use std::{sync::mpsc::{channel, Sender, Receiver}, thread::{spawn, JoinHandle}, time::Duration};

use anyhow::Result;
use nokhwa::{
    pixel_format::{RgbAFormat, RgbFormat},
    utils::{RequestedFormat, RequestedFormatType,CameraIndex::Index,Resolution},
    Camera,
    };


use slint::{Timer, TimerMode, Image};
const CANVAS_WIDTH: u32 = 1280;
const CANVAS_HEIGHT: u32 = 720;
const FPS:f32 = 30.0;


use slint::slint;
slint!{
    import {VerticalBox, HorizontalBox} from "std-widgets.slint";

export component Main inherits Window {
    title: "slint";
    icon: @image-url("");
    width: 1290px;
    height: 730px;

    pure callback render-image(int) -> image;
    in-out property <int> frame;

    VerticalLayout { 
        HorizontalLayout {
            alignment: center;
            Rectangle {
                border-color: white;
                border-width: 1px;
                width: 1280px;
                height: 720px;
                Image {
                    width: 100%;
                    height: 100%;
                    source: render-image(frame);
                }
            }
        }
    }
}

}

fn main() -> Result<()>{
    let window = Main::new().unwrap();
    
    let timer = Timer::default();
    let window_clone = window.as_weak();

    let (frame_sender, frame_receiver) = channel();
    let (exit_sender, exit_receiver) = channel();

    let mut frame_data = vec![0; (CANVAS_WIDTH * CANVAS_HEIGHT * 16) as usize];

    timer.start(TimerMode::Repeated, std::time::Duration::from_secs_f32(1./FPS), move || {
        if let Some(window) = window_clone.upgrade(){
            window.set_frame(window.get_frame()+1);
        }
    });

    let task = start(frame_sender, exit_receiver);

    let mut render = move || -> Result<Image>{

        if let Ok(pixels) = frame_receiver.try_recv(){
            
            frame_data = pixels;
    
        }

        let v = slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(
            &frame_data,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
        ));
        Ok(v)
    };

    window.on_render_image(move |_frame|{
        render().map_err(|err| eprintln!("{:?}", err)).unwrap()
    });

    window.run().unwrap();
    println!("Closed");
    exit_sender.send(())?;
    let result = task.join().unwrap();
    println!("Camera Stopped {:?}", result);
    Ok(())
}


fn start(frame_sender: Sender<Vec<u8>>, exit_receiver: Receiver<()>) -> JoinHandle<Result<()>>{
    
    spawn(move || -> Result<()>{
        let mut camera = Camera::new(
            Index(0), // index
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::HighestResolution(Resolution::new(1280, 720)))
        )
        .unwrap();
        camera.open_stream().unwrap();
        loop{
            if let Ok(()) = exit_receiver.try_recv(){
                break;
            }
            else{
            let frame_rgba = camera.frame().unwrap();            
            // Convert the image buffer to a dynamic image
            let pixels = frame_rgba.decode_image::<RgbAFormat>().unwrap();
            
            frame_sender.send(pixels.to_vec())?;

            std::thread::sleep(Duration::from_millis(10));
        }
        }
        Ok(())
    })
}