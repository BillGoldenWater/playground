use bytemuck::NoUninit;
use cgmath::{Array, ElementWise, MetricSpace, Vector2, vec2};
use itertools::Itertools as _;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, NoUninit)]
#[repr(C)]
pub struct Point {
    pub pos: [f32; 2],
    pub velocity: [f32; 2],
}

impl Point {
    pub fn generate() -> Vec<Self> {
        let boundary_size = 500_f64;
        let target_size = 250_f64;
        let spacing = 2.59_f64;
        let ball = true;
        let center = true;

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let num_per_axis = (target_size / spacing).floor() as u32;
        let size = f64::from(num_per_axis - 1) * spacing;

        let padding = (boundary_size - size) / 2.0;
        let padding = if center {
            Vector2::from_value(padding)
        } else {
            vec2(padding, 0.0)
        };

        let radius = size / 2.0;
        let center = padding.add_element_wise(radius);

        let points = (0..num_per_axis)
            .cartesian_product(0..num_per_axis)
            .map(|(x, y)| {
                vec2(x, y).map(f64::from).mul_element_wise(spacing)
                    + padding
            })
            .filter(|pos| !ball || pos.distance(center) <= radius)
            .map(|pos| Self {
                #[expect(clippy::cast_possible_truncation)]
                pos: [pos.x as f32, pos.y as f32],
                velocity: Default::default(),
            })
            .collect_vec();
        info!("generated {} points", points.len());
        points

        //let size = 500_f32;
        //let spacing = 2.59_f32;
        //let points_num = (size / spacing).powi(2);
        //let num = points_num.sqrt();
        //let spacing = size / num;
        //let half_spacing = spacing / 2.0;

        // let mut rng = rand::thread_rng();
        // let velocity_range = -100.0..100.0;

        // let step = 0.001;
        // let rotate_radians = std::f32::consts::PI * 2.0 * (step / 360.0);
        //
        // let center = vec2(20000.0, 20000.0);

        // let points = vec![
        //     Point {
        //         pos: [5000.0, 5000.0],
        //         velocity: [0.0, 0.0],
        //     },
        //     Point {
        //         pos: [5000.0, 5250.0],
        //         velocity: [0.0, 0.0],
        //     },
        // ];

        //(0..points_num as usize)
        //    .map(move |idx| {
        //        let idx = idx as f32;
        //        let x = (idx % num).floor() * spacing + half_spacing;
        //        let y = (idx / num).floor() * spacing + half_spacing;
        //        // let pos = vec2(x, y) - center;
        //        // let rotated = vec2(
        //        //     pos.x * rotate_radians.cos() - pos.y * rotate_radians.sin(),
        //        //     pos.x * rotate_radians.sin() + pos.y * rotate_radians.cos(),
        //        // );
        //        // let rotate = (rotated - pos) / step * 10.0;
        //        Point {
        //            velocity: [0.0, 0.0],
        //            // velocity: [rotate.x, rotate.y],
        //            // velocity: [
        //            //     rotate.x + rng.gen_range(velocity_range.clone()),
        //            //     rotate.y + rng.gen_range(velocity_range.clone()),
        //            // ],
        //            pos: [x, y], /* [
        //                             x as f32 + rng.gen_range(velocity_range.clone()),
        //                             y as f32 + rng.gen_range(velocity_range.clone()),
        //                         ] */
        //        }
        //    })
        //    .collect_vec()
    }
}
