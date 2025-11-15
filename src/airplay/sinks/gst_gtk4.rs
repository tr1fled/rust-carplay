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
use gtk::{gdk, gio, glib};
use gst_app::AppSrc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct GstSinkData {
    pub data_sink: Option<AppSrc>
}

impl GstSinkData {
    pub fn new() -> Self {
        Self {
            data_sink: None
        }
    }
}

fn create_ui(app: &gtk::Application, sinkdata: Arc<Mutex<GstSinkData>>) {
    let pipeline = gst::Pipeline::new();
    
    let h264parse = gst::ElementFactory::make("h264parse").build().unwrap();
    let decoder   = gst::ElementFactory::make("decodebin").build().unwrap();
    let convert   = gst::ElementFactory::make("videoconvert").build().unwrap();
    let gtksink   = gst::ElementFactory::make("gtk4paintablesink").build().unwrap();
    let queue = gst::ElementFactory::make("queue").build().unwrap();
    
    let paintable = gtksink.property::<gdk::Paintable>("paintable");
    //appsrc.push_buffer(Buffer::from_slice(slice));
    
    {
        let guard = sinkdata.lock().unwrap();
        let src = guard.data_sink.as_ref().expect("shouldn't be none");
        
        let caps = gst::Caps::builder("video/x-h264")
        .field("stream-format", "byte-stream")   // annex-b
        .field("alignment", "au")                // important
        .build();
        src.set_caps(Some(&caps));
        
        pipeline
        .add_many([src.upcast_ref(), &queue, &h264parse, &decoder, &convert, &gtksink])
        .unwrap();
        
        // Static links (OK)
        src.link(&queue).unwrap();
        queue.link(&h264parse).unwrap();
        h264parse.link(&decoder).unwrap();
        
        // ❗ DO NOT link decodebin statically
        // decoder.link(&convert).unwrap();   // REMOVE THIS
        
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
    /*let caps = gst_video::VideoCapsBuilder::new()
    .width(640)
    .height(480)
    .any_features()
    .build();
    {
    let guard = sinkdata.lock().unwrap();
    let src = guard.data_sink.as_ref().expect("shouldn't be none");
    src.link_filtered(&h264parse, &caps).unwrap();
    }*/
    
    
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1920, 1080);
    
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    
    let gst_widget = gstgtk4::RenderWidget::new(&gtksink);
    vbox.append(&gst_widget);
    
    let label = gtk::Label::new(Some("Position: 00:00:00"));
    vbox.append(&label);
    
    window.set_child(Some(&vbox));
    window.present();
    
    app.add_window(&window);
    
    let pipeline_weak = pipeline.downgrade();
    let timeout_id = glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let Some(pipeline) = pipeline_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        
        let position = pipeline.query_position::<gst::ClockTime>();
        label.set_text(&format!("Position: {:.0}", position.display()));
        glib::ControlFlow::Continue
    });
    
    let bus = pipeline.bus().unwrap();
    
    println!("PIPELINE HAS BEEN INIT! PIPELINE HAS BEEN INIT! PIPELINE HAS BEEN INIT! PIPELINE HAS BEEN INIT! PIPELINE HAS BEEN INIT! PIPELINE HAS BEEN INIT! ");
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

pub fn init_sink(some_data: Arc<Mutex<GstSinkData>>) -> glib::ExitCode {
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
