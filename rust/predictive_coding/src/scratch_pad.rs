// #[unsafe(no_mangle)]
// pub fn test(net: &mut Network) {
//     net.update_activities(false);
// }

// #[inline]
// pub fn fence() {
//     std::sync::atomic::compiler_fence(
//         std::sync::atomic::Ordering::SeqCst,
//     );
//     fence___________________________________();
// }
//
// #[unsafe(no_mangle)]
// #[inline(never)]
// pub fn fence___________________________________() {
//     std::hint::black_box(());
// }

// fn main() {
//     // activity, error, weight pred, weight err
//     let mut l0: (Fp, Fp, Fp, Fp) = (0.0, 0.0, 0.1, 0.1);
//     let mut l1: (Fp, Fp, Fp, Fp) = (0.0, 0.0, 0.1, 0.1);
//     let mut l2: (Fp, Fp, Fp, Fp) = (0.0, 0.0, 0.1, 0.1);
//     // l0.2 = rand::random_range(-0.1..0.1);
//     // l1.2 = rand::random_range(-0.1..0.1);
//     // l2.2 = rand::random_range(-0.1..0.1);
//     // let mut l0: (Fp, Fp, Fp) = rand::random();
//     // let mut l1: (Fp, Fp, Fp) = rand::random();
//     // let mut l2: (Fp, Fp, Fp) = rand::random();
//
//     let mut oi = (-1., 0.1);
//
//     let lr_a = 0.5;
//     let lr_w = 0.0001;
//     let a: fn(Fp) -> Fp = identity;
//
//     let mut guide = true;
//     let mut f_count = 0;
//
//     let mut count = 1;
//     loop {
//         if guide {
//             (l0.0, l2.0) = oi;
//         } else {
//             l2.0 = oi.1;
//         }
//
//         // l2.0 += (-l2.1) * lr_a;
//         l1.0 += (-l1.1 + (l2.1 * l1.3)) * lr_a;
//         l0.0 += (-l0.1 + (l1.1 * l0.3)) * lr_a;
//
//         l2.1 = a(l2.0) - (l2.2 * a(l1.0));
//         l1.1 = a(l1.0) - (l1.2 * a(l0.0));
//         // l0.1 = a(l0.0) - 0.0;
//
//         l2.2 += (l2.1 * a(l1.0)) * lr_w;
//         l1.2 += (l1.1 * a(l0.0)) * lr_w;
//         l0.2 += (0.0) * lr_w;
//
//         l2.3 += (0.0) * lr_w;
//         l1.3 += (l1.0 * a(l2.1)) * lr_w;
//         l0.3 += (l0.0 * a(l1.1)) * lr_w;
//
//         println!("{l0: >8.5?} {l1: >8.5?} {l2: >8.5?}");
//         stdout().flush().unwrap();
//
//         if !l1.0.is_finite() {
//             break;
//         }
//
//         // std::thread::sleep(Duration::from_secs_f64(0.005));
//
//         if (count > 10000 && l1.1.abs() < 0.001)
//             || (!guide && count > 100 && l1.1.abs() < 0.001)
//         {
//             oi.1 = rand::random_range(-0.1..0.1);
//             oi.0 = oi.1 * -10.;
//             if f_count > 100 {
//                 std::thread::sleep(Duration::from_secs_f64(1.));
//                 guide = false;
//             } else {
//                 f_count += 1;
//             }
//             count = 0;
//         }
//
//         count += 1;
//     }
// }

// fn main() {
//     let mut network = Network::new(0.01, 0.01, 1., 1., &[1, 1, 1]);
//
//     let mut io = (0.1, 1.);
//
//     let mut guide = true;
//     let mut learn = true;
//     let mut f_count = 0;
//
//     let mut count = 1;
//     loop {
//         network.input()[0] = io.0;
//         if guide {
//             network.output()[0] = io.1;
//         }
//
//         network.update_activities(true);
//         network.update_errors(true);
//         if learn {
//             network.update_weights();
//         }
//
//         let i = network.input()[0];
//         let e = network.error_avg();
//         let o = network.output()[0];
//         println!("{i: >13.10?} {e: >13.10?} {o: >13.10?}");
//         stdout().flush().unwrap();
//
//         if !o.is_finite() {
//             dbg!(&network);
//             break;
//         }
//
//         // std::thread::sleep(Duration::from_secs_f64(0.005));
//
//         if (count > 100 && e.abs() < 0.001)
//             || (!guide && count > 100 && e.abs() < 0.001)
//         {
//             io.0 = rand::random_range(0.0..0.1);
//             io.1 = io.0 * 10.;
//             if f_count > 100 {
//                 std::thread::sleep(Duration::from_secs_f64(1.));
//                 guide = false;
//                 learn = false;
//             } else {
//                 f_count += 1;
//             }
//             network.reset();
//             count = 0;
//             println!("reset");
//         }
//
//         count += 1;
//     }
// }

// fn activation_deriv(x: Fp) -> Fp {
//     1.
// }
// fn tanh_deriv(x: Fp) -> Fp {
//     1. - tanh(x).powi(2)
// }
