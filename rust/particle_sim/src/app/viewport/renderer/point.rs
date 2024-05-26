use bytemuck::NoUninit;
use cgmath::vec2;
use itertools::Itertools as _;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, NoUninit)]
#[repr(C)]
pub struct Point {
    pub pos: [f32; 2],
    pub velocity: [f32; 2],
}

impl Point {
    pub fn gen() -> Vec<Point> {
        let points_num = 35000_f32;
        let num = points_num.sqrt();
        let spacing = 40000.0 / num;
        let half_spacing = spacing / 2.0;

        let mut rng = rand::thread_rng();
        let velocity_range = -100.0..100.0;

        let step = 0.001;
        let rotate_radians = std::f32::consts::PI * 2.0 * (step / 360.0);

        let center = vec2(20000.0, 20000.0);

        let points = (0..points_num as usize)
            .into_iter()
            .map(move |idx| {
                let idx = idx as f32;
                let x = (idx % num).floor() * spacing + half_spacing;
                let y = (idx / num).floor() * spacing + half_spacing;
                let pos = vec2(x, y) - center;
                let rotated = vec2(
                    pos.x * rotate_radians.cos() - pos.y * rotate_radians.sin(),
                    pos.x * rotate_radians.sin() + pos.y * rotate_radians.cos(),
                );
                let rotate = (rotated - pos) / step * 10.0;
                Point {
                    velocity: [0.0, 0.0],
                    // velocity: [rotate.x, rotate.y],
                    // velocity: [
                    //     rotate.x + rng.gen_range(velocity_range.clone()),
                    //     rotate.y + rng.gen_range(velocity_range.clone()),
                    // ],
                    pos: [x, y], /* [
                                     x as f32 + rng.gen_range(velocity_range.clone()),
                                     y as f32 + rng.gen_range(velocity_range.clone()),
                                 ] */
                }
            })
            .collect_vec();

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

        points
    }
}
