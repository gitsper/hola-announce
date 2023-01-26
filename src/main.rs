use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::exit;

mod app_args;
mod experiments;
mod inputs;
mod utils;

use crate::app_args::parse_opts;
use crate::experiments::{run_bold_attempts, run_cautious_analysis};
use crate::inputs::{Announcements, MapfInstance, MapfSolution};
use crate::utils::{
    compute_kahead_announcements, compute_kgrouped_announcements, compute_robust_announcements,
    extend_stay_in_place, generate_plots,
};

fn main() {
    let opts = parse_opts();
    match opts.subcommand() {
        ("compute-secure-announcements", Some(_)) => {
            eprintln!("feature not implemented, exiting");
            exit(1);
        }
        ("analyze-attackers", Some(sub_c)) => {
            // read in instance file
            let instance_path = sub_c.value_of("mapf-instance").unwrap();
            let mut instance_file = match File::open(&instance_path) {
                Err(why) => panic!("couldn't open {}: {}", instance_path, why),
                Ok(file) => file,
            };
            let mut instance_yaml = String::new();
            match instance_file.read_to_string(&mut instance_yaml) {
                Err(why) => panic!("couldn't read {}: {}", instance_path, why),
                _ => {}
            };
            let instance: MapfInstance = match serde_yaml::from_str(&instance_yaml) {
                Err(why) => panic!("error parsing {}: {}", instance_path, why),
                Ok(instance) => instance,
            };

            // read in solution path
            let solution_path = sub_c.value_of("mapf-solution").unwrap();
            let mut solution_file = match File::open(&solution_path) {
                Err(why) => panic!("couldn't open {}: {}", solution_path, why),
                Ok(file) => file,
            };
            let mut solution_yaml = String::new();
            match solution_file.read_to_string(&mut solution_yaml) {
                Err(why) => panic!("couldn't read {}: {}", solution_path, why),
                _ => {}
            };
            let solution: MapfSolution = match serde_yaml::from_str(&solution_yaml) {
                Err(why) => panic!("error parsing {}: {}", solution_path, why),
                Ok(mut solution) => {
                    extend_stay_in_place(&mut solution);
                    solution
                }
            };

            // compute or read in announcements path
            let announcements: Announcements = match sub_c
                .value_of("announcement-strategy")
                .unwrap()
            {
                "kahead" => {
                    let k = sub_c
                        .value_of("lookahead")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if sub_c.is_present("skip-large") && (k > solution.statistics.makespan + 1) {
                        panic!("provided lookahead larger than makespan, skipping.");
                    }
                    compute_kahead_announcements(&instance.agents, k, solution.statistics.makespan)
                }
                "kgrouped" => {
                    let k = sub_c
                        .value_of("lookahead")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if sub_c.is_present("skip-large") && (k > solution.statistics.makespan + 1) {
                        panic!("provided lookahead larger than makespan, skipping.");
                    }
                    compute_kgrouped_announcements(
                        &instance.agents,
                        k,
                        solution.statistics.makespan,
                    )
                }
                "robust" => compute_robust_announcements(&instance, &solution),
                "custom" => {
                    let announcement_path = sub_c.value_of("custom-announcements").unwrap();
                    let mut announcement_file = match File::open(&announcement_path) {
                        Err(why) => panic!("couldn't open {}: {}", announcement_path, why),
                        Ok(file) => file,
                    };
                    let mut announcement_yaml = String::new();
                    match announcement_file.read_to_string(&mut announcement_yaml) {
                        Err(why) => panic!("couldn't read {}: {}", announcement_path, why),
                        _ => {}
                    };
                    match serde_yaml::from_str(&announcement_yaml) {
                        Err(why) => panic!("error parsing {}: {}", announcement_path, why),
                        Ok(announcement) => announcement,
                    }
                }
                _ => unreachable!(),
            };

            let output_path = sub_c.value_of("output").unwrap();
            {
                match OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output_path)
                {
                    Err(why) => panic!("couldn't touch {} for writing: {}", output_path, why),
                    _ => {}
                };
            }

            // run trials
            match sub_c.value_of("type").unwrap() {
                "bold" => {
                    let res = run_bold_attempts(
                        instance,
                        solution,
                        announcements,
                        !sub_c.is_present("no-mitigation"),
                    );
                    println!(
                        "{:>5} / {:>5} dangerous and undetected. {:.2} miss rate",
                        res.dangerous_undetected_count(),
                        res.attempts.len(),
                        res.miss_rate().unwrap_or(f64::NAN)
                    );
                    let output_yaml = serde_yaml::to_string(&res).ok().unwrap();
                    let mut output_file = match File::create(&output_path) {
                        Err(why) => panic!("couldn't open {}: {}", output_path, why),
                        Ok(file) => file,
                    };
                    match output_file.write_all(output_yaml.as_bytes()) {
                        Err(why) => panic!("error writing to {}: {}", output_path, why),
                        _ => {}
                    };
                }
                "cautious" => {
                    let res = run_cautious_analysis(instance, solution, announcements);
                    println!(
                        "{:>5} / {:>5} secure.",
                        res.secure_count(),
                        res.attempts.len(),
                    );
                    let output_yaml = serde_yaml::to_string(&res).ok().unwrap();
                    let mut output_file = match File::create(&output_path) {
                        Err(why) => panic!("couldn't open {}: {}", output_path, why),
                        Ok(file) => file,
                    };
                    match output_file.write_all(output_yaml.as_bytes()) {
                        Err(why) => panic!("error writing to {}: {}", output_path, why),
                        _ => {}
                    };
                }
                _ => unreachable!(),
            }
        }
        ("generate-plots", Some(sub_c)) => {
            generate_plots(
                sub_c.value_of("plot").unwrap(),
                sub_c.value_of("output").unwrap(),
                sub_c.is_present("force"),
            );
        }
        _ => println!("{}", opts.usage()),
    };
}
