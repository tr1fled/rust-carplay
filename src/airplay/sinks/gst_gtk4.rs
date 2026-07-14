// GST-GTK4 sink, made for working on any OS that supports Gstreamer and GTK4
//
// Very easy to operate too
// HOURS LATER:: THIS WILL NOT BE USED
// because i cannot be bothered to update to
// Debian 13 (Trixie) just for a higher GTK version

// and so we rewrite the GTK4 sink to be more complex
// but to have wider compatibility
use std::{
    thread,
    time::Duration,
};
use gst::{Buffer, prelude::*};
use gtk::prelude::*;
use std::sync::mpsc::{Receiver, Sender};
use gtk::{Button, ApplicationWindow, Box, Orientation, gdk, gio, glib, GestureClick, EventControllerMotion};
use std::rc::Rc;
use std::cell::Cell;
use gst_app::AppSrc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use gstgtk4::RenderWidget;
use serde::Serialize;

use crate::HID::touchscreen::hid_touchscreen_fill_report;
use crate::airplay::airplay_session::{AirplaySession, EventSessionMessage};
use crate::networking::http_client::send_command;

#[derive(Serialize)]
pub struct SetNightMode {
    pub r#type: String,
    pub nightMode: bool,
}

#[derive(Debug, Clone)]
pub struct GstSinkData {
    pub data_sink: Option<AppSrc>,
    
    // VERY lazy solution
    pub peer_ip: String
}

#[derive(Clone)]
struct ThrottledSender {
    latest: Rc<RefCell<Option<EventSessionMessage>>>,
    tx: Arc<Sender<EventSessionMessage>>,
}


impl GstSinkData {
    pub fn new() -> Self {
        Self {
            data_sink: None,
            peer_ip: String::new()
        }
    }
}

impl SetNightMode {
    pub fn new(nightMode: bool) -> Self {
        Self {
            r#type: String::from("setNightMode"),
            nightMode: false,
        }
    }
}

fn new_throttled_sender(tx: Arc<Sender<EventSessionMessage>>) -> ThrottledSender {
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

fn setup_mouse_handlers(widget: &RenderWidget, transmitter: Arc<Sender<EventSessionMessage>>) {
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

fn create_ui(app: &gtk::Application, tx: Arc<Sender<EventSessionMessage>>, src: Arc<AppSrc>) {
    let pipeline = gst::Pipeline::new();
    
    let h264parse = gst::ElementFactory::make("h264parse").build().unwrap();
    let decoder   = gst::ElementFactory::make("v4l2h264dec").build().unwrap();
    let convert   = gst::ElementFactory::make("v4l2convert").build().unwrap();
    let gtksink   = gst::ElementFactory::make("gtk4paintablesink").build().unwrap();
    let queue = gst::ElementFactory::make("queue").build().unwrap();

    queue.set_property("max-size-buffers", 1u32);
    queue.set_property("max-size-bytes", 0u32);
    queue.set_property("max-size-time", 0u64);
    queue.set_property_from_str("leaky", "downstream");
    //let queue = gst::ElementFactory::make("queue").build().unwrap();
    
    src.set_property("is-live", true);
    src.set_property("do-timestamp", true);
    src.set_property("format", gst::Format::Time);
    src.set_property("block", true);
    src.set_property("max-buffers", 1u64);
    
    gtksink.set_property("sync", &false);
    gtksink.set_property("max-lateness", &(-1i64));
    //appsrc.push_buffer(Buffer::from_slice(slice));
    let mut sinkdata: Arc<Mutex<GstSinkData>>;
    {
        
        let caps = gst::Caps::builder("video/x-h264")
        .field("stream-format", "byte-stream")   // annex-b
        .field("alignment", "au")                // important
        //.field("framerate", gst::Fraction::new(30, 1))
        .build();
        src.set_caps(Some(&caps));
        
        pipeline
        .add_many([src.upcast_ref(), &queue, &h264parse, &decoder, &convert, &gtksink])
        //.add_many([src.upcast_ref(), &queue, &h264parse, &decoder, &convert, &gtksink])
        .unwrap();
        
        // Static links (OK)
        //src.link(&queue).unwrap();
        //queue.link(&h264parse).unwrap();
        src.link(&h264parse).unwrap();
        //queue.link(&h264parse).unwrap();
        h264parse.link(&decoder).unwrap();
        decoder.link(&convert).unwrap();
        convert.link(&gtksink).unwrap();
    }
    
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1280, 720);
    
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    
    let gst_widget = gstgtk4::RenderWidget::new(&gtksink);
    setup_mouse_handlers(&gst_widget, tx.clone());
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
    
    // events
    // Create a button with label and margins
    let button = Button::builder()
    .label("🌓 Day/Night mode")
    .margin_top(10)
    .margin_bottom(10)
    .margin_start(12)
    .margin_end(12)
    .build();
    let button2 = Button::builder()
    .label("Open Siri")
    .margin_top(10)
    .margin_bottom(10)
    .margin_start(12)
    .margin_end(12)
    .build();
    
    // Connect to "clicked" signal of `button`
    let night_mode = Rc::new(RefCell::new(false));
    let night_mode_clone = night_mode.clone();
    let txc = tx.clone();
    button.connect_clicked(move |_| {
        let value = *night_mode_clone.borrow();
        
        let mut dta = Vec::new();
        plist::to_writer_binary(&mut dta, &SetNightMode::new(value));
        
        txc.send(EventSessionMessage {
            content_type: "application/x-apple-binary-plist".into(),
            data: dta,
        }).unwrap();
        
        *night_mode_clone.borrow_mut() = !value;
    });
    button2.connect_clicked(|button| {
        // Set the label to "Hello World!" after the button has been clicked on
        button.set_label("Hello World!");
    });
    
    // Create a window
    let window2 = ApplicationWindow::builder()
    .application(app)
    .title("My GTK App")
    .build();
    
    let vbox2 = Box::builder()
    .orientation(Orientation::Vertical)
    .spacing(10) // Optional: Add spacing between widgets
    .build();
    
    vbox2.append(&button);
    vbox2.append(&button2);
    window2.set_child(Some(&vbox2));
    
    // Present window
    window2.present();
    app.add_window(&window2);
}

pub fn init_sink(tx: Arc<Sender<EventSessionMessage>>, src: Arc<AppSrc>) -> glib::ExitCode {
    gtk::init().unwrap();
    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");
    
    let app = gtk::Application::new(None::<&str>, gio::ApplicationFlags::FLAGS_NONE);
    
    // init ui
    app.connect_activate(move |app| {
        // call create_ui with the extra argument
        create_ui(app, Arc::clone(&tx), Arc::clone(&src));
    });
    let res = app.run();
    
    unsafe {
        gst::deinit();
    }
    
    res
}
