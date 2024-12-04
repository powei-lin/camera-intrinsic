# camera-intrinsic-calibration
[![crate](https://img.shields.io/crates/v/camera-intrinsic-calibration.svg)](https://crates.io/crates/camera-intrinsic-calibration)

A pure rust camera intrinsic calibration library.

## CLI Usage
```sh
# install cli
cargo install camera-intrinsic-calibration

ccrs -h

# run intrinsic calibration on TUM vi dataset
# Download and untar
wget https://vision.in.tum.de/tumvi/exported/euroc/1024_16/dataset-calib-cam1_1024_16.tar
tar xvzf dataset-calib-cam1_1024_16.tar

# [Optional] export RUST_LOG=trace
ccrs dataset-calib-cam1_1024_16 --model eucm
```

## Supported formats
### Dataset format
* Euroc (default)
    ```
    dataset_root
    └── mav0
        └── cam0
            ├── data
            │   ├── {time_stamp}.png
            │   ├── {time_stamp}.png
            │   └── {time_stamp}.png
            └── data.csv
    ```
* General `--dataset-format general`
    ```
    dataset_root
    └── cam0
        ├── any_file_name.png
        ├── any_file_name.png
        └── any_file_name.png
    ```
### Camera models
* Extended Unified (EUCM)
* Extended Unified with Tangential (EUCMT)
* Unified Camera Model (UCM)
* Kannala Brandt (KB4) aka OpenCV Fisheye
* OpenCV (OPENCV5) aka `plumb_bob` in ROS
* F-theta (FTHETA) by NVidia

## Examples
```sh
cargo run -r --example convert_model
```

## Acknowledgements
Links:
* https://cvg.cit.tum.de/data/datasets/visual-inertial-dataset
* https://github.com/itt-ustutt/num-dual
* https://github.com/sarah-quinones/faer-rs

Papers:

* Kukelova, Zuzana, et al. "Radial distortion homography." Proceedings of the IEEE conference on computer vision and pattern recognition. 2015.

### TODO
* [ ] Multi-camera extrinsic
* [ ] More calibration info