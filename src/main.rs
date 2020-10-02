mod goal_mesh;
mod gradient;
mod half_edge;
mod utils;

use std::path::Path;

use crate::goal_mesh::GoalMesh;
use crate::gradient::Gradient;
use crate::utils::*;

use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use bevy_prototype_lyon::prelude::*;
use clap;
use log::info;

struct InputArgs {
    path_to_obj: String,
    resolution: u32,
    wireframe: bool,
}

fn main() {
    // Parse all of the commandline args
    let matches = clap::App::new("Unfold")
        .version("0.1")
        .author("Michael Walczyk")
        .about("📦 A program for unfolding arbitrary convex objects.")
        .short_flag('w')
        .long_flag("wireframe")
        .arg(
            clap::Arg::new("INPUT")
                .about("Sets the input .obj file, i.e. the goal mesh")
                .required(true),
        )
        .arg(
            clap::Arg::new("RESOLUTION")
                .about("Sets the resolution (width and height) of the renderer")
                .short('r')
                .long("resolution")
                .value_name("PIXELS")
                .default_value("1024")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("WIREFRAME")
                .about("Sets the draw mode to wireframe (instead of filled)")
                .short('w')
                .long("wireframe")
        )
        .get_matches();

    // This arg is required, so we can safely unwrap
    let path_to_obj = matches.value_of("INPUT").unwrap().to_owned();
    info!("Unfolding .obj: {:?}", path_to_obj);

    let resolution = matches
        .value_of("RESOLUTION")
        .unwrap()
        .parse::<u32>()
        .expect("Invalid resolution");
    info!(
        "Setting resolution to {:?}x{:?} pixels",
        resolution, resolution
    );

    // Aggregate args
    let input_args = InputArgs {
        path_to_obj,
        resolution,
        wireframe: matches.is_present("WIREFRAME"),
    };

    App::build()
        .add_resource(WindowDescriptor {
            width: resolution,
            height: resolution,
            title: String::from("durer"),
            ..Default::default()
        })
        .add_resource(ClearColor(Color::rgb(1.0, 0.98, 0.98)))
        .add_resource(Msaa { samples: 8 })
        .add_resource(input_args)
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    args: Res<InputArgs>,
) {
    let mut goal_mesh = GoalMesh::from_obj(&Path::new(&args.path_to_obj[..]), 0.into());
    let mut unfolded_positions = goal_mesh.unfold();

    let (net_size_x, net_size_y) = find_extents(&unfolded_positions);
    let padding = 100.0;
    let net_center = find_centroid(&unfolded_positions);
    let net_scale = (args.resolution as f32 - padding) / net_size_x.max(net_size_y);
    info!("Net size: {:?} x {:?}", net_size_x, net_size_y);
    info!("Net center: {:?}", net_center);

    for point in unfolded_positions.iter_mut() {
        *point = (*point - net_center) * net_scale;
    }

    // let gradient = Gradient::linear_spacing(&vec![
    //     Vec3::new(0.23921568627450981, 0.20392156862745098, 0.5450980392156862),
    //     Vec3::new(0.4627450980392157, 0.47058823529411764, 0.9294117647058824),
    //     Vec3::new(0.9686274509803922, 0.7215686274509804, 0.00392156862745098),
    //     Vec3::new(0.9450980392156862, 0.5294117647058824, 0.00392156862745098),
    //     Vec3::new(0.9529411764705882, 0.3568627450980392, 0.01568627450980392),
    // ]);

    // let mats = (0..5)
    //     .into_iter()
    //     .map(|i| {
    //         let c1 = colors[i];//gradient.color_at(i as f32 / 5.0);
    //         let c2 = Vec3::new(
    //             to_linear(c1.x()),
    //             to_linear(c1.y()),
    //             to_linear(c1.z())
    //         );
    //
    //         materials.add(Color::rgb(c2.x(), c2.y(), c2.z()).into())
    //     })
    //     .collect::<Vec<_>>();

    let colors = vec![
        Vec3::new(0.5568627450980392, 0.792156862745098, 0.9019607843137255),
        Vec3::new(0.12941176470588237, 0.6196078431372549, 0.7372549019607844),
        Vec3::new(0.00784313725490196, 0.18823529411764706, 0.2784313725490196),
        Vec3::new(1.0, 0.7176470588235294, 0.011764705882352941),
        Vec3::new(0.984313725490196, 0.5215686274509804, 0.0),
    ];

    let mats = colors
        .iter()
        .map(|color| {
            let color = Vec3::new(
                srgb_to_linear(color.x()),
                srgb_to_linear(color.y()),
                srgb_to_linear(color.z()),
            );
            materials.add(Color::rgb(color.x(), color.y(), color.z()).into())
        })
        .collect::<Vec<_>>();

    for triangle_index in 0..unfolded_positions.len() / 3 {
        let a = unfolded_positions[triangle_index * 3 + 0];
        let b = unfolded_positions[triangle_index * 3 + 1];
        let c = unfolded_positions[triangle_index * 3 + 2];

        let material = mats[triangle_index % mats.len()];

        let shape_type = ShapeType::Polyline {
            points: vec![
                (a.x(), a.y()).into(),
                (b.x(), b.y()).into(),
                (c.x(), c.y()).into(),
            ],
            closed: true,
        };

        let translation = Vec3::zero();

        if args.wireframe {
            commands.spawn(primitive(
                material,
                &mut meshes,
                shape_type,
                TessellationMode::Stroke(
                    &StrokeOptions::default()
                        .with_line_width(2.0)
                        .with_line_join(LineJoin::Round)
                        .with_line_cap(LineCap::Round),
                ),
                translation,
            ));
        } else {
            commands.spawn(primitive(
                material,
                &mut meshes,
                shape_type,
                TessellationMode::Fill(&FillOptions::default()),
                translation,
            ));
        }
    }

    // Add the camera
    commands.spawn(Camera2dComponents::default());
}
