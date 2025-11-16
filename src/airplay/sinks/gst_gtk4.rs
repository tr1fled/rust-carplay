// GST-GTK4 sink, made for working on any OS that supports Gstreamer and GTK4
//
// Very easy to operate too
// HOURS LATER:: THIS WILL NOT BE USED
// because i cannot be bothered to update to
// Debian 13 (Trixie) just for a higher GTK version

// and so we rewrite the GTK4 sink to be more complex
// but to have wider compatibility
use gst::{Buffer, prelude::*};
use gtk::prelude::*;
use std::sync::mpsc::Sender;
use gtk::{gdk, gio, glib, GestureClick, EventControllerMotion};
use std::rc::Rc;
use std::cell::Cell;
use gst_app::AppSrc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use gstgtk4::RenderWidget;

use crate::HID::touchscreen::hid_touchscreen_fill_report;
use crate::airplay::airplay_session::{AirplaySession, EventSessionMessage};
use crate::networking::http_client::send_command;

#[derive(Debug, Clone)]
pub struct GstSinkData {
    pub data_sink: Option<AppSrc>,
    
    // VERY lazy solution
    pub peer_ip: String
}

#[derive(Clone)]
struct ThrottledSender {
    latest: Rc<RefCell<Option<EventSessionMessage>>>,
    tx: Sender<EventSessionMessage>,
}


impl GstSinkData {
    pub fn new() -> Self {
        Self {
            data_sink: None,
            peer_ip: String::new()
        }
    }
}

fn new_throttled_sender(tx: Sender<EventSessionMessage>) -> ThrottledSender {
    let latest = Rc::new(RefCell::new(None));
    let latest_clone = latest.clone();
    let tx_clone = tx.clone();
    
    // Send at 60 Hz (16 ms)
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        if let Some(ev) = latest_clone.borrow_mut().take() {
            let _ = tx_clone.send(ev);
        }
        glib::ControlFlow::Continue
    });
    
    ThrottledSender { latest, tx }
}

fn setup_mouse_handlers(widget: &RenderWidget, transmitter: Sender<EventSessionMessage>) {
    // Shared state: is the left mouse button down?
    let mouse_down = Rc::new(Cell::new(false));
    
    //
    // CLICK HANDLER (mouse down + mouse up)
    //
    let click = GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY); // only left button
    let tx = new_throttled_sender(transmitter.clone());
    // ----- MOUSE DOWN -----
    {
        let mouse_down = mouse_down.clone();
        click.connect_pressed(move |_click, _n, x, y| {
            mouse_down.set(true);
            *tx.latest.borrow_mut() = Some(EventSessionMessage {
                content_type: "application/x-apple-binary-plist".into(),
                data: hid_touchscreen_fill_report(true, x.round() as u16, y.round() as u16),
            });
        });
    }
    let tx = new_throttled_sender(transmitter.clone());
    // ----- MOUSE UP -----
    {
        let mouse_down = mouse_down.clone();
        click.connect_released(move |_click, _n, x, y| {
            mouse_down.set(false);
            *tx.latest.borrow_mut() = Some(EventSessionMessage {
                content_type: "application/x-apple-binary-plist".into(),
                data: hid_touchscreen_fill_report(false, x as u16, y as u16),
            });
            println!("Mouse UP at {x}, {y}");
            // your code here (end drag, etc.)
        });
    }
    
    widget.add_controller(click);
    
    //
    // MOTION HANDLER (only active when mouse is down)
    //
    let tx = new_throttled_sender(transmitter.clone());
    let motion = EventControllerMotion::new();
    {
        let mouse_down = mouse_down.clone();
        
        motion.connect_motion(move |_motion, x, y| {
            if mouse_down.get() {
                println!("Mouse MOVE (while down) at {x}, {y}");
                *tx.latest.borrow_mut() = Some(EventSessionMessage {
                    content_type: "application/x-apple-binary-plist".into(),
                    data: hid_touchscreen_fill_report(true, x as u16, y as u16),
                });
                // your dragging logic here
            }
        });
    }
    
    widget.add_controller(motion);
}

fn create_ui(app: &gtk::Application, airplay_session: Arc<Mutex<AirplaySession>>) {
    let pipeline = gst::Pipeline::new();
    
    let h264parse = gst::ElementFactory::make("h264parse").build().unwrap();
    let decoder   = gst::ElementFactory::make("decodebin").build().unwrap();
    let convert   = gst::ElementFactory::make("videoconvert").build().unwrap();
    let gtksink   = gst::ElementFactory::make("gtk4paintablesink").build().unwrap();
    //let queue = gst::ElementFactory::make("queue").build().unwrap();
    
    let paintable = gtksink.property::<gdk::Paintable>("paintable");
    gtksink.set_property("sync", &false);
    gtksink.set_property("max-lateness", &(-1i64));
    //appsrc.push_buffer(Buffer::from_slice(slice));
    let mut sinkdata: Arc<Mutex<GstSinkData>>;
    {
        sinkdata = airplay_session.lock().unwrap().sink.as_ref().expect("Sink is none! Please fix.").clone();
    }
    {
        let guard = sinkdata.lock().unwrap();
        let src = guard.data_sink.as_ref().expect("shouldn't be none");
        
        let caps = gst::Caps::builder("video/x-h264")
        .field("stream-format", "byte-stream")   // annex-b
        .field("alignment", "au")                // important
        .build();
        src.set_caps(Some(&caps));
        
        pipeline
        .add_many([src.upcast_ref(), &h264parse, &decoder, &convert, &gtksink])
        //.add_many([src.upcast_ref(), &queue, &h264parse, &decoder, &convert, &gtksink])
        .unwrap();
        
        // Static links (OK)
        //src.link(&queue).unwrap();
        //queue.link(&h264parse).unwrap();
        src.link(&h264parse).unwrap();
        h264parse.link(&decoder).unwrap();
        
        let convert_clone = convert.clone();
        
        decoder.connect_pad_added(move |dbin, src_pad| {
            println!("decodebin added pad {:?}", src_pad);
            
            // Get the sink pad from videoconvert
            let sink_pad = match convert_clone.static_pad("sink") {
                Some(p) => p,
                None => {
                    eprintln!("videoconvert has no sink pad!");
                    return;
                },
            };
            
            if sink_pad.is_linked() {
                println!("sink pad already linked");
                return;
            }
            
            // Try linking dynamic decodebin pad to videoconvert
            match src_pad.link(&sink_pad) {
                Ok(_) => println!("Linked decodebin → videoconvert"),
                Err(err) => eprintln!("Failed to link decodebin src pad: {:?}", err),
            }
        });
        
        // Now link convert → gtksink
        convert.link(&gtksink).unwrap();
    }
    
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1920, 1080);
    
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    
    let gst_widget = gstgtk4::RenderWidget::new(&gtksink);
    {
        let ip = airplay_session.lock().unwrap().event_channel.as_ref().expect("Sender is not there!").clone();
        setup_mouse_handlers(&gst_widget, ip);
    }
    vbox.append(&gst_widget);
    
    window.set_child(Some(&vbox));
    window.present();
    
    app.add_window(&window);
    
    let pipeline_weak = pipeline.downgrade();
    let timeout_id = glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let Some(pipeline) = pipeline_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        glib::ControlFlow::Continue
    });
    
    let bus = pipeline.bus().unwrap();
    
    pipeline
    .set_state(gst::State::Playing)
    .expect("Unable to set the pipeline to the `Playing` state");
    
    let app_weak = app.downgrade();
    let bus_watch = bus
    .add_watch_local(move |_, msg| {
        use gst::MessageView;
        
        let Some(app) = app_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        
        match msg.view() {
            MessageView::Eos(..) => app.quit(),
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                app.quit();
            }
            _ => (),
        };
        
        glib::ControlFlow::Continue
    })
    .expect("Failed to add bus watch");
    
    let timeout_id = RefCell::new(Some(timeout_id));
    let pipeline = RefCell::new(Some(pipeline));
    let bus_watch = RefCell::new(Some(bus_watch));
    app.connect_shutdown(move |_| {
        window.close();
        
        drop(bus_watch.borrow_mut().take());
        if let Some(pipeline) = pipeline.borrow_mut().take() {
            pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
        }
        
        if let Some(timeout_id) = timeout_id.borrow_mut().take() {
            timeout_id.remove();
        }
    });
}

pub fn init_sink(some_data: Arc<Mutex<AirplaySession>>) -> glib::ExitCode {
    gtk::init().unwrap();
    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");
    
    let app = gtk::Application::new(None::<&str>, gio::ApplicationFlags::FLAGS_NONE);
    
    // init ui
    app.connect_activate(move |app| {
        // call create_ui with the extra argument
        create_ui(app, some_data.clone());
    });
    let res = app.run();
    
    unsafe {
        gst::deinit();
    }
    
    res
}
