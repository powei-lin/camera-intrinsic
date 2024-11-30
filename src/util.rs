use std::collections::HashMap;

use crate::detected_points::FrameFeature;

use super::camera_model::generic::GenericModel;
use super::camera_model::UCM;
use super::optimization::factors::*;
use log::debug;
use nalgebra as na;
use tiny_solver::loss_functions::HuberLoss;
use tiny_solver::Optimizer;

pub fn rtvec_to_na_dvec(
    rtvec: ((f64, f64, f64), (f64, f64, f64)),
) -> (na::DVector<f64>, na::DVector<f64>) {
    (
        na::dvector![rtvec.0 .0, rtvec.0 .1, rtvec.0 .2],
        na::dvector![rtvec.1 .0, rtvec.1 .1, rtvec.1 .2],
    )
}

fn set_problem_parameter_bound(
    problem: &mut tiny_solver::Problem,
    generic_camera: &GenericModel<f64>,
) {
    problem.set_variable_bounds("params", 0, 0.0, 10000.0);
    problem.set_variable_bounds("params", 1, 0.0, 10000.0);
    problem.set_variable_bounds("params", 2, 0.0, generic_camera.width());
    problem.set_variable_bounds("params", 3, 0.0, generic_camera.height());
    for (distortion_idx, (lower, upper)) in generic_camera.distortion_params_bound() {
        problem.set_variable_bounds("params", distortion_idx, lower, upper);
    }
}

pub fn convert_model(source_model: &GenericModel<f64>, target_model: &mut GenericModel<f64>) {
    let mut problem = tiny_solver::Problem::new();
    let edge_pixels = source_model.width().max(source_model.height()) as u32 / 100;
    let cost = ModelConvertFactor::new(source_model, target_model, edge_pixels, 3);
    problem.add_residual_block(
        cost.residaul_num(),
        vec![("x".to_string(), target_model.params().len())],
        Box::new(cost),
        Some(Box::new(HuberLoss::new(1.0))),
    );

    let camera_params = source_model.camera_params();
    let mut target_params_init = target_model.params();
    target_params_init.rows_mut(0, 4).copy_from(&camera_params);

    let initial_values =
        HashMap::<String, na::DVector<f64>>::from([("x".to_string(), target_params_init)]);

    // initialize optimizer
    let optimizer = tiny_solver::GaussNewtonOptimizer {};

    // distortion parameter bound
    set_problem_parameter_bound(&mut problem, &target_model);

    // optimize
    let result = optimizer.optimize(&problem, &initial_values, None);

    // save result
    let result_params = result.get("x").unwrap();
    target_model.set_params(result_params);
}

pub fn init_ucm(
    frame_feature0: &FrameFeature,
    frame_feature1: &FrameFeature,
    rvec0: &na::DVector<f64>,
    tvec0: &na::DVector<f64>,
    rvec1: &na::DVector<f64>,
    tvec1: &na::DVector<f64>,
    init_f: f64,
    init_alpha: f64,
) -> GenericModel<f64> {
    let half_w = frame_feature0.img_w_h.0 as f64 / 2.0;
    let half_h = frame_feature0.img_w_h.1 as f64 / 2.0;
    let init_params = na::dvector![init_f, init_f, half_w, half_h, init_alpha];
    let ucm_init_model = GenericModel::UCM(UCM::new(
        &init_params,
        frame_feature0.img_w_h.0,
        frame_feature0.img_w_h.1,
    ));

    let mut init_focal_alpha_problem = tiny_solver::Problem::new();
    let init_f_alpha = na::dvector![init_f, init_alpha];

    for (_, fp) in &frame_feature0.features {
        let cost = UCMInitFocalAlphaFactor::new(&ucm_init_model, &fp.p3d, &fp.p2d);
        init_focal_alpha_problem.add_residual_block(
            2,
            vec![
                ("params".to_string(), 2),
                ("rvec0".to_string(), 3),
                ("tvec0".to_string(), 3),
            ],
            Box::new(cost),
            Some(Box::new(HuberLoss::new(1.0))),
        );
    }

    for (_, fp) in &frame_feature1.features {
        let cost = UCMInitFocalAlphaFactor::new(&ucm_init_model, &fp.p3d, &fp.p2d);
        init_focal_alpha_problem.add_residual_block(
            2,
            vec![
                ("params".to_string(), 2),
                ("rvec1".to_string(), 3),
                ("tvec1".to_string(), 3),
            ],
            Box::new(cost),
            Some(Box::new(HuberLoss::new(1.0))),
        );
    }

    let initial_values = HashMap::<String, na::DVector<f64>>::from([
        ("params".to_string(), init_f_alpha),
        ("rvec0".to_string(), rvec0.clone()),
        ("tvec0".to_string(), tvec0.clone()),
        ("rvec1".to_string(), rvec1.clone()),
        ("tvec1".to_string(), tvec1.clone()),
    ]);

    // initialize optimizer
    let optimizer = tiny_solver::GaussNewtonOptimizer {};

    println!("init ucm init f {}", initial_values.get("params").unwrap());
    println!("init rvec0{}", initial_values.get("rvec0").unwrap());
    println!("init tvec0{}", initial_values.get("tvec0").unwrap());
    println!("init rvec1{}", initial_values.get("rvec1").unwrap());
    println!("init tvec1{}", initial_values.get("tvec1").unwrap());

    // optimize
    init_focal_alpha_problem.set_variable_bounds("params", 0, init_f / 3.0, init_f * 3.0);
    init_focal_alpha_problem.set_variable_bounds("params", 1, 1e-6, 1.0);
    let mut second_round_values =
        optimizer.optimize(&init_focal_alpha_problem, &initial_values, None);

    println!(
        "params after {:?}\n",
        second_round_values.get("params").unwrap()
    );
    println!("after rvec0{}", second_round_values.get("rvec0").unwrap());
    println!("after tvec0{}", second_round_values.get("tvec0").unwrap());
    println!("after rvec1{}", second_round_values.get("rvec1").unwrap());
    println!("after tvec1{}", second_round_values.get("tvec1").unwrap());
    // panic!("stop");

    let focal = second_round_values["params"][0];
    let alpha = second_round_values["params"][1];
    let ucm_all_params = na::dvector![focal, focal, half_w, half_h, alpha];
    let ucm_camera = GenericModel::UCM(UCM::new(
        &ucm_all_params,
        frame_feature0.img_w_h.0,
        frame_feature0.img_w_h.1,
    ));
    second_round_values.remove("params");
    calib_camera(
        &[frame_feature0.clone(), frame_feature1.clone()],
        &ucm_camera,
    )
    .0
}

pub fn calib_camera(
    frame_feature_list: &[FrameFeature],
    generic_camera: &GenericModel<f64>,
) -> (GenericModel<f64>, Vec<(na::DVector<f64>, na::DVector<f64>)>) {
    let params = generic_camera.params();
    let params_len = params.len();
    let mut problem = tiny_solver::Problem::new();
    let mut initial_values =
        HashMap::<String, na::DVector<f64>>::from([("params".to_string(), params)]);
    debug!("init {:?}", initial_values);
    let mut valid_indexes = Vec::new();
    for (i, frame_feature) in frame_feature_list.iter().enumerate() {
        let mut p3ds = Vec::new();
        let mut p2ds = Vec::new();
        let rvec_name = format!("rvec{}", i);
        let tvec_name = format!("tvec{}", i);
        for (_, fp) in &frame_feature.features {
            let cost = ReprojectionFactor::new(&generic_camera, &fp.p3d, &fp.p2d);
            problem.add_residual_block(
                2,
                vec![
                    ("params".to_string(), params_len),
                    (rvec_name.clone(), 3),
                    (tvec_name.clone(), 3),
                ],
                Box::new(cost),
                Some(Box::new(HuberLoss::new(1.0))),
            );
            p3ds.push(fp.p3d);
            p2ds.push(na::Vector2::new(fp.p2d.x as f64, fp.p2d.y as f64));
        }
        let undistorted = generic_camera.unproject(&p2ds);
        let (p3ds, p2ds_z): (Vec<_>, Vec<_>) = undistorted
            .iter()
            .zip(p3ds)
            .filter_map(|(p2, p3)| {
                if let Some(p2) = p2 {
                    Some((p3, glam::Vec2::new(p2.x as f32, p2.y as f32)))
                } else {
                    None
                }
            })
            .unzip();
        // if p3ds.len() < 6 {
        //     println!("skip frame {}", i);
        //     continue;
        // }
        valid_indexes.push(i);
        let (rvec, tvec) =
            rtvec_to_na_dvec(sqpnp_simple::sqpnp_solve_glam(&p3ds, &p2ds_z).unwrap());

        if !initial_values.contains_key(&rvec_name) {
            initial_values.insert(rvec_name, rvec);
        }
        if !initial_values.contains_key(&tvec_name) {
            initial_values.insert(tvec_name, tvec);
        }
    }

    let optimizer = tiny_solver::GaussNewtonOptimizer {};
    let initial_values = optimizer.optimize(&problem, &initial_values, None);

    set_problem_parameter_bound(&mut problem, &generic_camera);
    let mut result = optimizer.optimize(&problem, &initial_values, None);

    let new_params = result.get("params").unwrap();
    println!("params {}", new_params);
    let mut calibrated_camera = generic_camera.clone();
    calibrated_camera.set_params(&new_params);
    let rtvec_vec: Vec<_> = valid_indexes
        .iter()
        .map(|&i| {
            let rvec_name = format!("rvec{}", i);
            let tvec_name = format!("tvec{}", i);
            (
                result.remove(&rvec_name).unwrap(),
                result.remove(&tvec_name).unwrap(),
            )
        })
        .collect();
    (calibrated_camera, rtvec_vec)
}
