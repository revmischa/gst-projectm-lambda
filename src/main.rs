use anyhow::Error;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSinkCallbacks};
use log::{debug, error, info, warn};
use std::env;
use tokio::runtime::Runtime;

fn visualize_audio() -> Result<(), Error> {
    // Initialize GStreamer
    gst::init()?;

    // Retrieve input and output file paths from environment variables
    let input_file = env::var("INPUT_AUDIO_FILE").unwrap_or_else(|_| "input.mp3".to_string());
    let output_file = env::var("OUTPUT_VIDEO_FILE").unwrap_or_else(|_| "output.mp4".to_string());

    // Build the pipeline
    let pipeline_str = format!(
        "filesrc location={} ! decodebin ! audioconvert ! audioresample ! appsink name=audio_sink \
         projectm name=visualizer ! videoconvert ! x264enc ! mp4mux name=mux ! filesink location={}",
        input_file, output_file
    );

    let pipeline = gst::parse::launch(&pipeline_str)?;

    // Get the appsink element
    let pipeline = pipeline.dynamic_cast::<gst::Bin>().unwrap();
    let appsink = pipeline
        .by_name("audio_sink")
        .unwrap()
        .dynamic_cast::<AppSink>()
        .unwrap();

    // Set up a callback to log when appsink receives data
    appsink.set_callbacks(
        AppSinkCallbacks::builder()
            .new_sample(|appsink| {
                let sample = appsink.pull_sample().unwrap();
                let buffer = sample.buffer().unwrap();
                let map = buffer.map_readable().unwrap();
                info!("Appsink received buffer of size: {}", map.size());
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    // Log the constructed pipeline
    debug!("Pipeline: {:?}", pipeline);

    // Start playing
    info!("Starting the pipeline");
    match pipeline.set_state(gst::State::Playing) {
        Ok(_) => info!("Pipeline is now playing"),
        Err(err) => {
            error!(
                "Unable to set the pipeline to the `Playing` state: {:?}",
                err
            );
            return Err(err.into());
        }
    }

    // Wait until error or EOS or Paused
    let bus = pipeline.bus().unwrap();
    let mut eos_received = false;

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => {
                info!("End of stream reached");
                eos_received = true;
                break;
            }
            MessageView::Error(err) => {
                error!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if let Some(element) = state_changed.src() {
                    debug!(
                        "State changed in element {:?} from {:?} to {:?}",
                        element.path_string(),
                        state_changed.old(),
                        state_changed.current()
                    );

                    if element == pipeline.dynamic_cast_ref::<gst::Element>().unwrap()
                        && state_changed.current() == gst::State::Paused
                    {
                        warn!("Pipeline is paused unexpectedly, checking further...");
                        // Check the state of individual elements
                        let mut iter = pipeline.iterate_elements().into_iter();
                        while let Some(Ok(elem)) = iter.next() {
                            debug!(
                                "Element {:?} is in state {:?}",
                                elem.path_string(),
                                elem.state(gst::ClockTime::NONE)
                            );
                        }
                        pipeline.set_state(gst::State::Playing)?;
                    }
                }
            }
            MessageView::Buffering(buffering) => {
                info!("Buffering {}%", buffering.percent());
                if buffering.percent() < 100 {
                    info!("Pipeline is buffering, pausing");
                    pipeline.set_state(gst::State::Paused)?;
                } else {
                    info!("Buffering complete, resuming playback");
                    pipeline.set_state(gst::State::Playing)?;
                }
            }
            MessageView::Latency(..) => {
                info!("Latency updated");
                if let Some(bin) = pipeline.dynamic_cast_ref::<gst::Bin>() {
                    bin.recalculate_latency().unwrap();
                }
            }
            MessageView::StreamStatus(status) => {
                debug!("Stream status changed: {:?}", status);
            }
            MessageView::DurationChanged(..) => {
                info!("Duration changed");
            }
            MessageView::ClockLost(..) => {
                warn!("Clock lost, setting state to Playing");
                pipeline.set_state(gst::State::Playing).unwrap();
            }
            _ => {
                debug!("Received message: {:?}", msg);
            }
        }
    }

    if !eos_received {
        warn!("No EOS received, pipeline may not have completed");
    }

    // Shutdown pipeline
    info!("Shutting down the pipeline");
    match pipeline.set_state(gst::State::Null) {
        Ok(_) => info!("Pipeline is now null"),
        Err(err) => {
            error!("Unable to set the pipeline to the `Null` state: {:?}", err);
        }
    }

    Ok(())
}

fn main() {
    // Initialize the logger
    env_logger::init();

    // Create a new runtime
    let runtime = Runtime::new().expect("Failed to create runtime");

    // Run the main function
    runtime.block_on(async {
        if let Err(err) = visualize_audio() {
            error!("Error occurred: {:?}", err);
        }
    });
}
