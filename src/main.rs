mod colormap;
mod colorramp;
mod julia;

use std::{
    ops::Add,
    path::{Path, PathBuf},
};

use colormap::{ColormapInputImage, ColormapMappingImage, ColormapOutputImage, ColormapPlugin};
use colorramp::ColorRamp;
use julia::{JuliaData, JuliaPlugin};

use bevy::{
    asset::FileAssetIo,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::render_resource::*,
    window::{WindowDescriptor, WindowResized},
};

use csv;
use palette::{rgb::Rgba, FromColor, Pixel};
use std::fs::File;

use bevy_better_exit::{BetterExitPlugin, ExitEvent, ExitListener};

struct MovementPath(Vec<Vec2>);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(JuliaPlugin)
        .add_plugin(ColormapPlugin::with_previous("julia"))
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(BetterExitPlugin::new(None))
        .add_startup_system(setup)
        .add_startup_system(load_path)
        .add_system(modi)
        .add_system(window_size)
        .add_system(bevy_better_exit::exit_on_esc_system)
        //.add_system(update_color)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut params: ResMut<Assets<JuliaData>>,
) {
    let mut julia_image = Image::new_fill(
        Extent3d {
            width: 400,
            height: 400,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(0.0 as f32).to_ne_bytes(),
        TextureFormat::R32Float,
    );
    julia_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let julia_image = images.add(julia_image);

    let mut mapped_image = Image::new_fill(
        Extent3d {
            width: 400,
            height: 400,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    mapped_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let mapped_image = images.add(mapped_image);

    let mut ramp = ColorRamp::new();
    ramp.add(0.0, 0.0, 0.0, 0.0, 1.0);
    ramp.add(0.04, 0.0, 0.0, 0.0, 1.0);
    ramp.add(0.2, 0.4, 0.0, 0.0, 1.0);
    ramp.add(0.5, 1.0, 0.4, 0.0, 1.0);
    ramp.add(0.8, 1.0, 1.0, 0.0, 1.0);
    ramp.add(1.0, 1.0, 1.0, 1.0, 1.0);

    /*let mut ramp = ColorRamp::new();
    ramp.add(0.00, 0.0, 0.0, 0.0, 1.0);
    ramp.add(0.02, 0.0, 0.0, 0.0, 1.0);
    ramp.add(0.05, 0.0, 0.0, 0.4, 1.0);
    ramp.add(0.1, 0.0, 0.3, 1.0, 1.0);
    ramp.add(0.15, 0.6, 0.6, 1.0, 1.0);
    ramp.add(0.2, 1.0, 1.0, 0.0, 1.0);
    ramp.add(0.3, 1.0, 0.4, 0.0, 1.0);
    ramp.add(0.4, 0.6, 0.0, 0.0, 1.0);
    ramp.add(1.0, 0.0, 0.0, 0.0, 1.0);*/

    let data = ramp.build_texture_data(1024, 1).unwrap();

    let mut mapping_image = Image::new_fill(
        Extent3d {
            width: 1024,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D1,
        &[128, 64, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    mapping_image.data = data;
    mapping_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let mapping_image = images.add(mapping_image);

    let data = JuliaData {
        c: Vec2::new(0.2, 0.3),
        view_aspect: 1.0,
        view_center: Vec2::new(0.0, 0.0),
        view_scale: 0.5,
        iters: 128,
        image: julia_image.clone(),
    };
    let data = params.add(data);

    commands.spawn_bundle(SpriteBundle {
        texture: mapped_image.clone(),
        ..Default::default()
    });

    commands.insert_resource(ColormapInputImage(julia_image));
    commands.insert_resource(ColormapOutputImage(mapped_image));
    commands.insert_resource(ColormapMappingImage(mapping_image));

    commands.spawn_bundle(Camera2dBundle::default());

    commands.insert_resource(data);
}

fn load_path(mut commands: Commands, server: Res<AssetServer>) {
    let mut path = PathBuf::new();
    path.push(FileAssetIo::get_base_path());
    path.push("assets");
    path.push("path.csv");
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(File::open(path).unwrap());

    let mut points: Vec<Vec2> = Vec::new();

    for r in reader.records() {
        let record = r.unwrap();
        let data = record.deserialize::<[f32; 2]>(None).unwrap();
        let x = data[0] / 2822.22217 * 4. - 2.5;
        let y = data[1] / 2822.22217 * 4. - 2.;
        points.push(Vec2::new(x, y));
    }

    commands.insert_resource(MovementPath(points));
}

fn modi(
    mut params: ResMut<Assets<JuliaData>>,
    data: Res<Handle<JuliaData>>,
    time: Res<Time>,
    mut path: Res<MovementPath>,
) {
    let frame = (time.seconds_since_startup() * 60.) as usize;
    let frame_wrap = frame % (path.0.len());
    let data = params.get_mut(&data).unwrap();
    data.c = path.0[frame_wrap];
    /*let av = am.ahead.process(ef.0);
    println!("av: {}", av);
    //data.c.x = (0.2 * av) * (0.7 * time.seconds_since_startup() as f32).cos() as f32;
    //data.c.y = (0.2 * av) * (0.9 * time.seconds_since_startup() as f32).sin() as f32;
    data.c.x = -1.0 + 0.3 * (time.seconds_since_startup() as f32).cos();
    data.c.y = 0.3 * (time.seconds_since_startup() as f32).sin();*/
}

fn window_size(
    mut size_event: EventReader<WindowResized>,
    julia: Res<Handle<JuliaData>>,
    output: Res<ColormapOutputImage>,
    mut images: ResMut<Assets<Image>>,
    mut params: ResMut<Assets<JuliaData>>,
) {
    for wse in size_event.iter() {
        let julia = params.get_mut(&julia).unwrap();
        julia.view_aspect = wse.width / wse.height;

        let julia_img = images.get_mut(&julia.image).unwrap();
        julia_img.resize(Extent3d {
            width: wse.width as u32,
            height: wse.height as u32,
            depth_or_array_layers: 1,
        });

        let output = images.get_mut(&output.0).unwrap();
        output.resize(Extent3d {
            width: wse.width as u32,
            height: wse.height as u32,
            depth_or_array_layers: 1,
        });
    }
}

/*fn update_color(
    mut images: ResMut<Assets<Image>>,
    colormap: Res<ColormapMappingImage>,
    time: Res<Time>,
) {
    let colormap = images.get_mut(&colormap.0).unwrap();

    let av = am.ahead.process(ef.0) * 10.;
    let sat = (av / 2.0).min(1.0);

    let cols = vec![
        palette::Hsla::from_components((0.0 as f32, sat, 0.5, 1.0)),
        palette::Hsla::from_components((32.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((64.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((96.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((128.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((160.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((192.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((224.0, sat, 0.5, 1.0)),
        palette::Hsla::from_components((256.0, sat, 0.5, 1.0)),
    ];
    let grad = palette::gradient::Gradient::new(cols);
    let mut data: Vec<_> = grad
        .take(1024)
        .map(|hsva: palette::Hsla| Rgba::from_color(hsva))
        .collect();

    let rotations = 750;
    println!("av: {}", av);

    let c = &mut data.drain(0..rotations).collect();
    data.append(c);

    let bytes = data
        .iter()
        .flat_map(|rgba| rgba.into_format().into_raw::<[u8; 4]>())
        .collect();

    colormap.data = bytes;
}
*/
