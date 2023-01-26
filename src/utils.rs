use decorum::N64;
use defaultmap::{DefaultBTreeMap, DefaultHashMap};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use plotters::prelude::*;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Read};
use std::iter;

use crate::experiments::{BoldExperimentResult, CautiousExperimentResult};
use crate::inputs::{Agent, Announcements, Coordinate, MapfInstance, MapfSolution};

pub fn extend_stay_in_place(solution: &mut MapfSolution) {
    for (_, path) in solution.schedule.iter_mut() {
        while path.len() <= solution.statistics.makespan + 1 {
            path.push(path.last().unwrap().as_time(path.last().unwrap().t + 1));
        }
    }
}

pub fn compute_kahead_announcements(
    agents: &Vec<Agent>,
    lookahead: usize,
    makespan: usize,
) -> Announcements {
    let mut schedule: HashMap<String, Vec<usize>> = HashMap::new();
    for agent in agents {
        // non-inclusive lookahead
        schedule.insert(
            agent.name.clone(),
            (lookahead + 1..lookahead + makespan + 2).collect(),
        );
    }
    Announcements { schedule: schedule }
}

pub fn compute_kgrouped_announcements(
    agents: &Vec<Agent>,
    lookahead: usize,
    makespan: usize,
) -> Announcements {
    let mut schedule: HashMap<String, Vec<usize>> = HashMap::new();
    for agent in agents {
        schedule.insert(
            agent.name.clone(),
            (lookahead + 1..lookahead + makespan + 2)
                .step_by(lookahead)
                .map(|elt| iter::repeat(elt).take(lookahead))
                .flatten()
                .collect(),
        );
    }
    Announcements { schedule: schedule }
}

pub fn compute_robust_announcements(
    instance: &MapfInstance,
    solution: &MapfSolution,
) -> Announcements {
    let mut schedule: HashMap<String, Vec<usize>> = HashMap::new();
    for agent in &instance.agents {
        schedule.insert(agent.name.clone(), Vec::new());
    }
    for t in 0..solution.statistics.makespan + 1 {
        // announcement at time t
        for agent in &instance.agents {
            // agent prefix
            if t > 0 && schedule[&agent.name][t - 1] > t + 1 {
                // agent is waiting for another to leave a conflict location
                let prev_announce = schedule[&agent.name][t - 1];
                schedule.get_mut(&agent.name).unwrap().push(prev_announce);
                continue;
            }
            'conflict_loop: for fut_t in (t + 2)..solution.statistics.makespan + 1 {
                for s in (t + 1)..fut_t {
                    for conflict_agent in &instance.agents {
                        if conflict_agent.name != agent.name
                            && Coordinate::from(solution.schedule[&agent.name][fut_t])
                                == Coordinate::from(solution.schedule[&conflict_agent.name][s])
                        {
                            // conflict_agent will be at the location before agent,
                            // we need to wait for conflict_agent to leave before announcing
                            // reaching this location
                            schedule.get_mut(&agent.name).unwrap().push(fut_t);
                            break 'conflict_loop;
                        }
                    }
                    // no conflict found at time s
                }
            }
            // if there was no conflict, just push the makespan and be done with
            if schedule[&agent.name].len() < t + 1 {
                schedule
                    .get_mut(&agent.name)
                    .unwrap()
                    .push(solution.statistics.makespan + 1);
            }
        }
    }
    Announcements { schedule: schedule }
}

pub fn generate_plots(plot: &str, plot_path: &str, force: bool) {
    println!("generating {} to \"{}\"", plot, plot_path);
    println!("force: {}", force);
    {
        match OpenOptions::new()
            .write(true)
            .create(true)
            .create_new(!force)
            .open(plot_path)
        {
            Err(why) => panic!("couldn't touch {} for writing: {}", plot_path, why),
            _ => {}
        };
    }
    let outputs: Vec<String> = io::stdin()
        .lock()
        .lines()
        .map(|line| line.unwrap())
        .collect();
    let pb = ProgressBar::new(outputs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{eta:>5}->{elapsed:>5}] [{wide_bar}] {pos:>7}/{len:>7}")
            .progress_chars("=> "),
    );
    match plot {
        "succ-vs-kahead" => generate_plot_succ_vs_kahead(plot_path, outputs, pb),
        "succ-vs-max-inter-obs" => generate_plot_succ_vs_max_inter_obs(plot_path, outputs, pb),
        "succ-vs-kgrouped" => generate_plot_succ_vs_kgrouped(plot_path, outputs, pb),
        "succ-vs-robust" => generate_plot_succ_vs_robust(plot_path, outputs, pb),
        "secure-vs-kahead" => generate_plot_secure_vs_kahead(plot_path, outputs, pb),
        "secure-vs-max-inter-obs" => generate_plot_secure_vs_max_inter_obs(plot_path, outputs, pb),
        "secure-vs-kgrouped" => generate_plot_secure_vs_kgrouped(plot_path, outputs, pb),
        "secure-vs-robust" => generate_plot_secure_vs_robust(plot_path, outputs, pb),
        _ => Err("unreachable".into()),
    }
    .expect("plotting error");
}
fn generate_plot_secure_vs_kahead(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut secure_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = DefaultBTreeMap::new(Vec::new());
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: CautiousExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        // each experiment result has the same map and announcements
        let secure_rate = experiment_result.secure_rate();
        let kahead = experiment_result
            .attempts
            .iter()
            .next()
            .unwrap()
            .min_lookahead;
        secure_vs_kahead[kahead].push(secure_rate.into());
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("Secure Rate vs Lookahead", ("Times", 18))
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("fixed lookahead")
        .y_desc("secure ratio")
        .draw()?;
    let data: Vec<_> = secure_vs_kahead
        .iter()
        .map(|(kahead, secure_rates)| {
            (
                *kahead as u64,
                (secure_rates.iter().map(|x| x.into_inner()).sum::<f64>()
                    / secure_rates.len() as f64),
            )
        })
        .collect();
    chart.draw_series(LineSeries::new(
        secure_vs_kahead.iter().map(|(kahead, secure_rates)| {
            (
                *kahead as u64,
                (secure_rates.iter().map(|x| x.into_inner()).sum::<f64>()
                    / secure_rates.len() as f64),
            )
        }),
        Into::<ShapeStyle>::into(&BLACK).stroke_width(1),
    ))?;
    println!("DATA secure-vs-kahead {}: {:?}", plot_path, data);
    println!("done.");
    Ok(())
}
fn generate_plot_secure_vs_max_inter_obs(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut secure_vs_max_inter_obs: DefaultBTreeMap<usize, DefaultHashMap<usize, [f64; 2]>> =
        DefaultBTreeMap::new(DefaultHashMap::new([0f64, 0f64]));
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: CautiousExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        for attempt in experiment_result.attempts.iter() {
            if attempt.max_inter_observation_time < 42 {
                secure_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][0] += if attempt.secured { 1f64 } else { 0f64 };
                secure_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][1] += 1f64;
            }
        }
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("Secure Rate vs Inter-Observation Time", ("Times", 18))
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("attacker max inter-observation time")
        .y_desc("secure ratio")
        .draw()?;
    // TODO: one line per lookahead
    let data: Vec<_> = secure_vs_max_inter_obs
        .iter()
        .map(|(miot, secure_count_by_lookahead)| {
            (*miot as u64, {
                let summed = secure_count_by_lookahead
                    .values()
                    .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                summed.0 / summed.1
            })
        })
        .collect();
    chart.draw_series(LineSeries::new(
        secure_vs_max_inter_obs
            .iter()
            .map(|(miot, secure_count_by_lookahead)| {
                (*miot as u64, {
                    let summed = secure_count_by_lookahead
                        .values()
                        .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                    summed.0 / summed.1
                })
            }),
        Into::<ShapeStyle>::into(&BLACK).stroke_width(1),
    ))?;
    println!("DATA secure-vs-max-inter-obs {}: {:?}", plot_path, data);
    println!("done.");
    Ok(())
}
fn generate_plot_secure_vs_kgrouped(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut secure_vs_kgrouped: DefaultBTreeMap<usize, Vec<N64>> = DefaultBTreeMap::new(Vec::new());
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: CautiousExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        // each experiment result has the same map and announcements
        let secure_rate = experiment_result.secure_rate();
        let min_inter_announcement_time = experiment_result
            .attempts
            .iter()
            .next()
            .unwrap()
            .min_inter_announcement_time;
        secure_vs_kgrouped[min_inter_announcement_time].push(secure_rate.into());
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("Secure Rate vs Inter-Announcement Time", ("Times", 18))
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("inter-announcement time")
        .y_desc("secure ratio")
        .draw()?;
    let data: Vec<_> = secure_vs_kgrouped
        .iter()
        .map(|(kahead, secure_rates)| {
            (
                *kahead as u64,
                (secure_rates.iter().map(|x| x.into_inner()).sum::<f64>()
                    / secure_rates.len() as f64),
            )
        })
        .collect();
    chart.draw_series(LineSeries::new(
        secure_vs_kgrouped.iter().map(|(kahead, secure_rates)| {
            (
                *kahead as u64,
                (secure_rates.iter().map(|x| x.into_inner()).sum::<f64>()
                    / secure_rates.len() as f64),
            )
        }),
        Into::<ShapeStyle>::into(&BLACK).stroke_width(1),
    ))?;
    println!("DATA secure-vs-kgrouped {}: {:?}", plot_path, data);
    println!("done.");
    Ok(())
}

fn generate_plot_secure_vs_robust(
    _plot_path: &str,
    _outputs: Vec<String>,
    _pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!();
    //Ok(())
}
fn generate_plot_succ_vs_kahead(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    //let mut succ_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = DefaultBTreeMap::new(Vec::new());
    let mut succ_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut miss_rate_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut false_alarm_rate_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut attempt_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: BoldExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        // each experiment result has the same map and announcements
        let succ_rate = experiment_result.attack_success_rate();
        let miss_rate = experiment_result.miss_rate();
        let false_alarm_rate = experiment_result.false_alarm_rate();
        let attempt_rate = experiment_result.attack_attempt_rate();
        let kahead = experiment_result
            .attempts
            .iter()
            .next()
            .unwrap()
            .min_lookahead;
        succ_vs_kahead[kahead].push(succ_rate.into());
        miss_rate.and_then(|x| Some(miss_rate_vs_kahead[kahead].push(x.into())));
        false_alarm_rate.and_then(|x| Some(false_alarm_rate_vs_kahead[kahead].push(x.into())));
        attempt_vs_kahead[kahead].push(attempt_rate.into());
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Balanced Bold Attacker Behavior vs Fixed Lookahead",
            ("Times", 18),
        )
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("fixed lookahead")
        .draw()?;
    chart
        .draw_series(LineSeries::new(
            succ_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(0)).stroke_width(1),
        ))?
        .label("attacker success ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(0)));
    println!(
        "DATA attack-succ-vs-kahead {}: {:?}",
        plot_path,
        succ_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            attempt_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(1)).stroke_width(1),
        ))?
        .label("attacker attempt ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(1)));
    println!(
        "DATA attempt-vs-kahead {}: {:?}",
        plot_path,
        attempt_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            false_alarm_rate_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(2)).stroke_width(1),
        ))?
        .label("early alarm ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(2)));
    println!(
        "DATA early-alarm-vs-kahead {}: {:?}",
        plot_path,
        false_alarm_rate_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            miss_rate_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(3)).stroke_width(1),
        ))?
        .label("miss ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(3)));
    println!(
        "DATA miss-rate-vs-kahead {}: {:?}",
        plot_path,
        miss_rate_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    println!("done.");
    Ok(())
}
fn generate_plot_succ_vs_max_inter_obs(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut succ_vs_max_inter_obs: DefaultBTreeMap<usize, DefaultHashMap<usize, [f64; 2]>> =
        DefaultBTreeMap::new(DefaultHashMap::new([0f64, 0f64]));
    let mut miss_rate_vs_max_inter_obs: DefaultBTreeMap<usize, DefaultHashMap<usize, [f64; 2]>> =
        DefaultBTreeMap::new(DefaultHashMap::new([0f64, 0f64]));
    let mut false_alarm_rate_vs_max_inter_obs: DefaultBTreeMap<
        usize,
        DefaultHashMap<usize, [f64; 2]>,
    > = DefaultBTreeMap::new(DefaultHashMap::new([0f64, 0f64]));
    let mut attempt_vs_max_inter_obs: DefaultBTreeMap<usize, DefaultHashMap<usize, [f64; 2]>> =
        DefaultBTreeMap::new(DefaultHashMap::new([0f64, 0f64]));
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: BoldExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        for attempt in experiment_result.attempts.iter() {
            if attempt.max_inter_observation_time < 42 {
                succ_vs_max_inter_obs[attempt.max_inter_observation_time][attempt.min_lookahead]
                    [0] += if attempt.dangerous { 1f64 } else { 0f64 };
                succ_vs_max_inter_obs[attempt.max_inter_observation_time][attempt.min_lookahead]
                    [1] += 1f64;
                miss_rate_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][0] += if attempt.dangerous && !attempt.detected {
                    1f64
                } else {
                    0f64
                };
                miss_rate_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][1] += if attempt.dangerous { 1f64 } else { 0f64 };
                false_alarm_rate_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][0] += if !attempt.dangerous && attempt.detected {
                    1f64
                } else {
                    0f64
                };
                false_alarm_rate_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][1] += if !attempt.dangerous { 1f64 } else { 0f64 };
                attempt_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][0] += if attempt.attempted() { 1f64 } else { 0f64 };
                attempt_vs_max_inter_obs[attempt.max_inter_observation_time]
                    [attempt.min_lookahead][1] += 1f64;
            }
        }
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Balanced Bold Attacker vs Inter-Observation Time",
            ("Times", 18),
        )
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("attacker max inter-observation time")
        .draw()?;
    chart
        .draw_series(LineSeries::new(
            succ_vs_max_inter_obs
                .iter()
                .map(|(miot, count_by_lookahead)| {
                    (*miot as u64, {
                        let summed = count_by_lookahead
                            .values()
                            .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                        summed.0 / summed.1
                    })
                }),
            Into::<ShapeStyle>::into(&Palette99::pick(0)).stroke_width(1),
        ))?
        .label("attacker success ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(0)));
    println!(
        "DATA succ-vs-max-inter-obs {}: {:?}",
        plot_path,
        succ_vs_max_inter_obs
            .iter()
            .map(|(miot, count_by_lookahead)| {
                (*miot as u64, {
                    let summed = count_by_lookahead
                        .values()
                        .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                    summed.0 / summed.1
                })
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            attempt_vs_max_inter_obs
                .iter()
                .map(|(miot, count_by_lookahead)| {
                    (*miot as u64, {
                        let summed = count_by_lookahead
                            .values()
                            .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                        summed.0 / summed.1
                    })
                }),
            Into::<ShapeStyle>::into(&Palette99::pick(1)).stroke_width(1),
        ))?
        .label("attacker attempt ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(1)));
    chart
        .draw_series(LineSeries::new(
            false_alarm_rate_vs_max_inter_obs
                .iter()
                .map(|(miot, count_by_lookahead)| {
                    (*miot as u64, {
                        let summed = count_by_lookahead
                            .values()
                            .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                        summed.0 / summed.1
                    })
                }),
            Into::<ShapeStyle>::into(&Palette99::pick(2)).stroke_width(1),
        ))?
        .label("early alarm ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(2)));
    chart
        .draw_series(LineSeries::new(
            miss_rate_vs_max_inter_obs
                .iter()
                .map(|(miot, count_by_lookahead)| {
                    (*miot as u64, {
                        let summed = count_by_lookahead
                            .values()
                            .fold((0f64, 0f64), |acc, x| (acc.0 + x[0], acc.1 + x[1]));
                        summed.0 / summed.1
                    })
                }),
            Into::<ShapeStyle>::into(&Palette99::pick(3)).stroke_width(1),
        ))?
        .label("miss ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(3)));
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    println!("done.");
    Ok(())
}

fn generate_plot_succ_vs_kgrouped(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut succ_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut miss_rate_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut false_alarm_rate_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    let mut attempt_vs_kahead: DefaultBTreeMap<usize, Vec<N64>> = Default::default();
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: BoldExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        // each experiment result has the same map and announcements
        let succ_rate = experiment_result.attack_success_rate();
        let miss_rate = experiment_result.miss_rate();
        let false_alarm_rate = experiment_result.false_alarm_rate();
        let attempt_rate = experiment_result.attack_attempt_rate();
        let min_inter_announcement_time = experiment_result
            .attempts
            .iter()
            .next()
            .unwrap()
            .min_inter_announcement_time;
        succ_vs_kahead[min_inter_announcement_time].push(succ_rate.into());
        miss_rate
            .and_then(|x| Some(miss_rate_vs_kahead[min_inter_announcement_time].push(x.into())));
        false_alarm_rate.and_then(|x| {
            Some(false_alarm_rate_vs_kahead[min_inter_announcement_time].push(x.into()))
        });
        attempt_vs_kahead[min_inter_announcement_time].push(attempt_rate.into());
    }
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Balanced Bold Attacker vs Inter-Announcement Time",
            ("Times", 18),
        )
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0u64..45u64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("inter-announcement time")
        //.y_desc("detection ratio")
        .draw()?;
    chart
        .draw_series(LineSeries::new(
            succ_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(0)).stroke_width(1),
        ))?
        .label("attacker success ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(0)));
    println!(
        "DATA succ-vs-kgrouped {}: {:?}",
        plot_path,
        succ_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            attempt_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(1)).stroke_width(1),
        ))?
        .label("attacker attempt ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(1)));
    println!(
        "DATA attempt-vs-kgrouped {}: {:?}",
        plot_path,
        attempt_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            false_alarm_rate_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(2)).stroke_width(1),
        ))?
        .label("early alarm ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(2)));
    println!(
        "DATA early-alarm-vs-kgrouped {}: {:?}",
        plot_path,
        false_alarm_rate_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .draw_series(LineSeries::new(
            miss_rate_vs_kahead.iter().map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            }),
            Into::<ShapeStyle>::into(&Palette99::pick(3)).stroke_width(1),
        ))?
        .label("miss ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(3)));
    println!(
        "DATA miss-vs-kgrouped {}: {:?}",
        plot_path,
        miss_rate_vs_kahead
            .iter()
            .map(|(kahead, rates)| {
                (
                    *kahead as u64,
                    rates.iter().map(|x| x.into_inner()).sum::<f64>() / rates.len() as f64,
                )
            })
            .collect::<Vec<_>>()
    );
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    println!("done.");
    Ok(())
}

fn generate_plot_succ_vs_robust(
    plot_path: &str,
    outputs: Vec<String>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut succ_vs_robust: Vec<(f64, f64)> = Default::default();
    let mut miss_rate_vs_robust: Vec<(f64, f64)> = Default::default();
    let mut false_alarm_rate_vs_robust: Vec<(f64, f64)> = Default::default();
    let mut attempt_vs_robust: Vec<(f64, f64)> = Default::default();
    println!("parsing output files...");
    for output_path in outputs.iter().progress_with(pb) {
        let mut output_file = match File::open(&output_path) {
            Err(why) => panic!("couldn't open {}: {}", output_path, why),
            Ok(file) => file,
        };
        let mut output_yaml = String::new();
        match output_file.read_to_string(&mut output_yaml) {
            Err(why) => panic!("couldn't read {}: {}", output_path, why),
            _ => {}
        };
        let experiment_result: BoldExperimentResult = match serde_yaml::from_str(&output_yaml) {
            Err(why) => panic!("error parsing {}: {}", output_path, why),
            Ok(output) => output,
        };
        // each experiment result has the same map and announcements
        let succ_rate = experiment_result.attack_success_rate();
        let miss_rate = experiment_result.miss_rate();
        let false_alarm_rate = experiment_result.false_alarm_rate();
        let attempt_rate = experiment_result.attack_attempt_rate();
        let robust = experiment_result
            .attempts
            .iter()
            .next()
            .unwrap()
            .avg_lookahead
            .unwrap();
        succ_vs_robust.push((robust.into(), succ_rate));
        miss_rate.and_then(|x| Some(miss_rate_vs_robust.push((robust.into(), x))));
        false_alarm_rate.and_then(|x| Some(false_alarm_rate_vs_robust.push((robust.into(), x))));
        attempt_vs_robust.push((robust.into(), attempt_rate));
    }
    println!(
        "average lookahead was {} with miss rate {}",
        succ_vs_robust.iter().map(|(x, y)| x).sum::<f64>() / (succ_vs_robust.len() as f64),
        succ_vs_robust.iter().map(|(x, y)| y).sum::<f64>() / (succ_vs_robust.len() as f64)
    );
    println!("plotting...");
    let root = SVGBackend::new(plot_path, (516, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Balanced Bold Attacker Behavior vs Robust Announcement",
            ("Times", 18),
        )
        .margin(10)
        .margin_right(20)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(0f64..45f64, 0f64..1.1f64)?;
    chart
        .configure_mesh()
        .x_label_style(("Times", 16))
        .y_label_style(("Times", 16))
        .x_desc("Average Lookahead")
        .draw()?;
    chart
        .draw_series(PointSeries::of_element(
            succ_vs_robust,
            1,
            ShapeStyle::from(&Palette99::pick(0)).filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))?
        .label("attacker success ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(0)));
    chart
        .draw_series(PointSeries::of_element(
            attempt_vs_robust,
            1,
            ShapeStyle::from(&Palette99::pick(1)).filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))?
        .label("attacker attempt ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(1)));
    chart
        .draw_series(PointSeries::of_element(
            false_alarm_rate_vs_robust,
            1,
            ShapeStyle::from(&Palette99::pick(2)).filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))?
        .label("early alarm ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(2)));
    chart
        .draw_series(PointSeries::of_element(
            miss_rate_vs_robust,
            1,
            ShapeStyle::from(&Palette99::pick(3)).filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))?
        .label("miss ratio")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &Palette99::pick(3)));
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    println!("done.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::{
        Agent, Coordinate, Map, MapfInstance, MapfSolution, Statistics, TimedCoordinate,
    };
    use std::collections::{HashMap, HashSet};

    #[test]
    fn min_inter_announcement_time() {
        let agents: Vec<Agent> = (0..100)
            .map(|i| Agent {
                goal: Coordinate { x: i, y: i },
                start: Coordinate { x: i, y: i },
                name: i.to_string(),
            })
            .collect();
        let announcements = compute_kgrouped_announcements(&agents, 10, 100);
        assert_eq!(10, announcements.min_inter_announcement_time());
    }
    #[test]
    fn min_lookahead() {
        let agents: Vec<Agent> = (0..100)
            .map(|i| Agent {
                goal: Coordinate { x: i, y: i },
                start: Coordinate { x: i, y: i },
                name: i.to_string(),
            })
            .collect();
        let announcements = compute_kahead_announcements(&agents, 10, 100);
        assert_eq!(10, announcements.min_lookahead());
    }
    #[test]
    fn robust_announcement_two_agents_following_each_other() {
        let instance = MapfInstance {
            agents: vec![
                Agent {
                    name: "agent0".to_string(),
                    start: Coordinate { x: 1, y: 3 },
                    goal: Coordinate { x: 9, y: 9 },
                },
                Agent {
                    name: "agent1".to_string(),
                    start: Coordinate { x: 1, y: 0 },
                    goal: Coordinate { x: 8, y: 8 },
                },
                Agent {
                    name: "agent2".to_string(),
                    start: Coordinate { x: 3, y: 3 },
                    goal: Coordinate { x: 7, y: 7 },
                },
            ],
            map: Map {
                dimensions: Coordinate { x: 10, y: 10 },
                obstacles: HashSet::new(),
            },
        };
        let mut schedule: HashMap<String, Vec<TimedCoordinate>> = Default::default();
        schedule.insert(
            "agent0".to_string(),
            vec![
                TimedCoordinate { x: 1, y: 3, t: 0 },
                TimedCoordinate { x: 1, y: 3, t: 1 },
                TimedCoordinate { x: 2, y: 3, t: 2 },
                TimedCoordinate { x: 2, y: 3, t: 3 },
                TimedCoordinate { x: 3, y: 3, t: 4 },
                TimedCoordinate { x: 3, y: 3, t: 5 },
            ],
        );
        schedule.insert(
            "agent1".to_string(),
            vec![
                TimedCoordinate { x: 1, y: 0, t: 0 },
                TimedCoordinate { x: 1, y: 1, t: 1 },
                TimedCoordinate { x: 1, y: 2, t: 2 },
                TimedCoordinate { x: 1, y: 3, t: 3 },
                TimedCoordinate { x: 1, y: 4, t: 4 },
                TimedCoordinate { x: 1, y: 5, t: 5 },
            ],
        );
        schedule.insert(
            "agent2".to_string(),
            vec![
                TimedCoordinate { x: 3, y: 3, t: 0 },
                TimedCoordinate { x: 3, y: 2, t: 1 },
                TimedCoordinate { x: 3, y: 3, t: 2 },
                TimedCoordinate { x: 3, y: 2, t: 3 },
                TimedCoordinate { x: 3, y: 2, t: 4 },
                TimedCoordinate { x: 3, y: 2, t: 5 },
            ],
        );
        let solution = MapfSolution {
            schedule: schedule,
            statistics: Statistics {
                cost: 0,
                makespan: 5,
                runtime: 0.0,
                highLevelExpanded: 0,
                lowLevelExpanded: 0,
            },
        };
        println!("{:?}", compute_robust_announcements(&instance, &solution));
        assert!(false);
    }
}
