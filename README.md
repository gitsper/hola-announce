## README
 * * *

This repository contains the source code to reproduce the experiments from our
paper, "HoLA Robots: Mitigating Plan-Deviation Attacks in Multi-Robot Systems
with Co-Observations and Horizon-Limiting Announcements"

### Usage

Build the project with `cargo build --release`

The `./experiments/inputs/` contains benchmark grid maps and precomputed MAPF
plans. See `./experiments/run_*.sh` for sample usage and
`./experiments/gen_plots.sh` for plotting results.
