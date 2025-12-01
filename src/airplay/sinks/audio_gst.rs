use gst::{Buffer, glib, prelude::*};
use gst_sys as ffi;
use gst_app::AppSrc;
use std::sync::{Mutex, Arc};

use crate::airplay::airplay_session::AudioStream;

pub fn init_sink(in_stream: Arc<Mutex<AudioStream>>) {
    // make sure gstreamer is initialized, if not, init
    if unsafe { ffi::gst_is_initialized() } != glib::ffi::GTRUE {
        gst::init().unwrap();
    }
    let pipeline = gst::Pipeline::default();
    // caps & appsrc initialization
    let codec_data = gst::Buffer::from_slice(vec![0x12, 0x10]);
    let caps = gst::Caps::builder("audio/mpeg")
    .field("mpegversion", 4) // <-- AnnexB
    .field("stream-format", "raw")
    .field("channels", 2)
    .field("rate", 44100)
    .field("codec_data", codec_data)
    .build();
    let appsrc = AppSrc::builder()
    .caps(&caps)
    .format(gst::Format::Time)
    .build();
    appsrc.set_property("format", &gst::Format::Time);
    appsrc.set_property("is-live", &true);
    appsrc.set_property("do-timestamp", &true);
    {
        let mut is = in_stream.lock().unwrap(); 
        is.sink = Some(appsrc.clone()); 
        is.pipeline = Some(pipeline.clone());
    }
    // faad ! audioconvert ! audioresample ! autoaudiosink
    let aacparse = gst::ElementFactory::make("aacparse")
    .build()
    .unwrap();
    let faad = gst::ElementFactory::make("faad")
    .build()
    .unwrap();
    let convert = gst::ElementFactory::make("audioconvert")
    .build()
    .unwrap(); 
    let audioresample = gst::ElementFactory::make("audioresample")
    .build()
    .unwrap();
    let sink = gst::ElementFactory::make("autoaudiosink")
    .build()
    .unwrap();  
    pipeline.add_many([appsrc.upcast_ref(), &aacparse, &faad, &convert, &audioresample, &sink]).unwrap();
    gst::Element::link_many([appsrc.upcast_ref(), &aacparse, &faad, &convert, &audioresample, &sink]).unwrap();
    pipeline
    .set_state(gst::State::Playing)
    .expect("Unable to set the pipeline to the `Playing` state");
    let bus = pipeline
    .bus()
    .expect("Pipeline without bus. Shouldn't happen!");
}