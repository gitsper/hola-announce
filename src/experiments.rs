use decorum::N64;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use itertools::Itertools;
use matrix_display::{cell, matrix, style, Format, MatrixDisplay};
use petgraph::algo::astar;
use petgraph::graphmap::DiGraphMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::io::Read;

use crate::inputs::{Announcements, Coordinate, MapfInstance, MapfSolution, TimedCoordinate};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BoldAttemptResult {
    pub attacker_name: String,
    pub safe: Coordinate,
    pub dangerous: bool,          // attacker reached safe
    pub detected: bool,           // deviation detected
    pub max_deviated_dist: usize, // maximum distance the attacker deviated from the nominal
    pub max_inter_observation_time: usize,
    pub min_inter_announcement_time: usize,
    pub min_lookahead: usize,
    pub avg_lookahead: Option<N64>,
}

impl BoldAttemptResult {
    pub fn attempted(&self) -> bool {
        self.max_deviated_dist > 0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BoldExperimentResult {
    pub attempts: Vec<BoldAttemptResult>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CautiousAttemptResult {
    pub attacker_name: String,
    pub safe: Coordinate,
    pub max_inter_observation_time: usize,
    pub min_inter_announcement_time: usize,
    pub min_lookahead: usize,
    pub avg_lookahead: Option<N64>,
    pub secured: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CautiousExperimentResult {
    pub attempts: Vec<CautiousAttemptResult>,
}

impl CautiousExperimentResult {
    pub fn secure_count(&self) -> usize {
        self.attempts.iter().filter(|a| a.secured).count()
    }
    pub fn secure_rate(&self) -> f64 {
        (self.secure_count() as f64) / (self.attempts.len() as f64)
    }
}

impl BoldExperimentResult {
    pub fn attempted_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.attempted())
            .count()
    }
    pub fn dangerous_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.dangerous)
            .count()
    }
    pub fn non_dangerous_count(&self) -> usize {
        self.attempts.len() - self.dangerous_count()
    }
    pub fn dangerous_undetected_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| attempt.dangerous && !attempt.detected)
            .count()
    }
    pub fn non_dangerous_detected_count(&self) -> usize {
        self.attempts
            .iter()
            .filter(|attempt| !attempt.dangerous && attempt.detected)
            .count()
    }
    pub fn attack_success_rate(&self) -> f64 {
        // percentage of positives in the population
        self.dangerous_count() as f64 / self.attempts.len() as f64
    }
    pub fn attack_attempt_rate(&self) -> f64 {
        // percentage of attack attempts in the population
        self.attempted_count() as f64 / self.attempts.len() as f64
    }
    pub fn miss_rate(&self) -> Option<f64> {
        // miss rate
        if self.dangerous_count() > 0 {
            Some(self.dangerous_undetected_count() as f64 / self.dangerous_count() as f64)
        } else {
            None
        }
    }
    pub fn false_alarm_rate(&self) -> Option<f64> {
        // false alarm rate, what's the chance I issue the alarm and the attacker isn't dangerous
        // (yet).  FP / N
        if self.non_dangerous_count() > 0 {
            Some(self.non_dangerous_detected_count() as f64 / self.non_dangerous_count() as f64)
        } else {
            None
        }
    }
}

pub fn run_cautious_analysis(
    instance: MapfInstance,
    solution: MapfSolution,
    announcements: Announcements,
) -> CautiousExperimentResult {
    let pb = ProgressBar::new(100u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{eta:>5}->{elapsed:>5}] [{wide_bar}] {pos:>7}/{len:>7}")
            .progress_chars("=> "),
    );
    CautiousExperimentResult {
        attempts: instance
            .agents
            .iter()
            .take(10)
            .cartesian_product(instance.map.obstacles.iter().take(10))
            .par_bridge()
            .progress_with(pb)
            .map(|(attacker, safe)| {
                run_cautious_attempt(&instance, &solution, &announcements, &attacker.name, safe)
            })
            .collect(),
    }
}

fn run_cautious_attempt(
    instance: &MapfInstance,
    solution: &MapfSolution,
    announcements: &Announcements,
    attacker_name: &String,
    safe: &Coordinate,
) -> CautiousAttemptResult {
    let g = build_graph(&instance, &solution, safe);
    let mut res = CautiousAttemptResult {
        attacker_name: attacker_name.clone(),
        safe: *safe,
        max_inter_observation_time: solution.max_inter_observation_time(&attacker_name),
        min_inter_announcement_time: announcements.min_inter_announcement_time(),
        min_lookahead: announcements.min_lookahead(),
        avg_lookahead: Some(announcements.avg_lookahead()),
        secured: true,
    };
    let mut c: HashSet<TimedCoordinate> = Default::default();
    for t in 0..solution.statistics.makespan + 1 {
        // time, agent name, reachable set
        let mut x: HashMap<usize, HashMap<String, HashSet<Coordinate>>> = Default::default();
        let mut s: usize = 0;
        x.insert(t, Default::default());
        for agent in &instance.agents {
            let mut init_set = HashSet::new();
            init_set.insert(solution.schedule[&agent.name][t].into());
            x.get_mut(&t).unwrap().insert(agent.name.clone(), init_set);
        }
        'outer: while x[&(t + s)][attacker_name]
            .is_disjoint(&defender_observed(&x[&(t + s)], attacker_name))
        {
            //println!("{}: {} + {}", attacker_name, t, s);
            //print_board(&x[&(t + s)], attacker_name, instance);
            x.insert(t + s + 1, x[&(t + s)].clone());
            for agent in &instance.agents {
                let new_flood = match reachable(
                    solution,
                    announcements,
                    &g,
                    x.get(&(t + s + 1)).unwrap().get(&agent.name).unwrap(),
                    t,     // to check what the announcements are
                    t + s, // to check in the plan
                    &mut c,
                    false,
                ) {
                    Ok(flood) => flood,
                    Err(_) => {
                        // println!("found conflict, restarting");
                        s = 0;
                        continue 'outer;
                    }
                };
                x.get_mut(&(t + s + 1))
                    .unwrap()
                    .insert(agent.name.clone(), new_flood);
            }
            let diff_defender = x[&(t + s + 1)][attacker_name]
                .difference(
                    &x[&(t + s)]
                        .iter()
                        .filter(|(name, _)| *name != attacker_name)
                        .map(|(_, flood)| flood)
                        .fold(HashSet::new(), |acc, elt| {
                            acc.union(&elt).cloned().collect()
                        }),
                )
                .cloned()
                .collect();
            x.get_mut(&(t + s + 1))
                .unwrap()
                .insert(attacker_name.clone(), diff_defender);
            for defender in instance
                .agents
                .iter()
                .filter(|agent| agent.name != *attacker_name)
            {
                let diff_attacker = x[&(t + s + 1)][&defender.name]
                    .difference(&x[&(t + s + 1)][attacker_name])
                    .cloned()
                    .collect();
                x.get_mut(&(t + s + 1))
                    .unwrap()
                    .insert(defender.name.clone(), diff_attacker);
            }
            let diff_defender_next = x[&(t + s + 1)][attacker_name]
                .difference(
                    &x[&(t + s + 1)]
                        .iter()
                        .filter(|(name, _)| *name != attacker_name)
                        .map(|(_, flood)| flood)
                        .fold(HashSet::new(), |acc, elt| {
                            acc.union(&elt).cloned().collect()
                        }),
                )
                .cloned()
                .collect();
            x.get_mut(&(t + s + 1))
                .unwrap()
                .insert(attacker_name.clone(), diff_defender_next);
            if x[&(t + s)] == x[&(t + s + 1)] {
                // no progress
                res.secured = false;
                return res;
            }
            s = s + 1;
        }
        if x[&(t + s)][attacker_name]
            .intersection(&defender_observed(&x[&(t + s)], attacker_name))
            .take(1) // how many potential observations to check
            .map(|&p| {
                attack_exists(
                    solution,
                    announcements,
                    attacker_name,
                    safe,
                    t,
                    t + s,
                    p,
                    &g,
                    &x,
                    &mut c,
                )
            })
            .all(|b| b)
        {
            res.secured = false;
            return res;
        }
    }
    res
}

fn attack_exists(
    solution: &MapfSolution,
    announcements: &Announcements,
    attacker_name: &String,
    safe: &Coordinate,
    start_time: usize,
    end_time: usize,
    obs_coord: Coordinate,
    g: &DiGraphMap<TimedCoordinate, ()>,
    x: &HashMap<usize, HashMap<String, HashSet<Coordinate>>>,
    conflicts: &mut HashSet<TimedCoordinate>,
) -> bool {
    let mut a = x[&start_time][attacker_name].clone();
    let mut b: HashSet<Coordinate> = Default::default();
    for u in start_time..end_time {
        a = reachable(
            solution,
            announcements,
            g,
            &a,
            start_time,
            u,
            conflicts,
            true,
        )
        .unwrap();
        b = reachable(
            solution,
            announcements,
            g,
            &b,
            start_time,
            u,
            conflicts,
            true,
        )
        .unwrap();
        a = a
            .difference(&defender_observed(&x[&u], attacker_name))
            .cloned()
            .collect();
        b = b
            .difference(&defender_observed(&x[&u], attacker_name))
            .cloned()
            .collect();
        if a.contains(&safe) {
            b.insert(safe.clone());
        }
        if a.len() == 0 && b.len() == 0 {
            break;
        }
    }
    b.contains(&obs_coord)
}

fn reachable(
    solution: &MapfSolution,
    announcements: &Announcements,
    g: &DiGraphMap<TimedCoordinate, ()>,
    flood: &HashSet<Coordinate>,
    curr_time: usize,
    fut_time: usize,
    conflicts: &mut HashSet<TimedCoordinate>,
    attacker_mode: bool,
) -> Result<HashSet<Coordinate>, ()> {
    let mut new_flood: HashSet<Coordinate> = HashSet::new();
    for v in flood.iter() {
        new_flood = new_flood
            .union(
                &(match move_robot(
                    solution,
                    announcements,
                    g,
                    *v,
                    curr_time,
                    fut_time,
                    conflicts,
                    attacker_mode,
                ) {
                    Ok(f) => Ok(f),
                    Err(()) => {
                        if attacker_mode {
                            Ok(HashSet::new())
                        } else {
                            Err(())
                        }
                    }
                }?),
            )
            .cloned()
            .collect();
    }
    Ok(new_flood)
}

fn move_robot(
    solution: &MapfSolution,
    announcements: &Announcements,
    g: &DiGraphMap<TimedCoordinate, ()>,
    coord: Coordinate,
    curr_time: usize,
    fut_time: usize,
    conflicts: &mut HashSet<TimedCoordinate>,
    attacker_mode: bool,
) -> Result<HashSet<Coordinate>, ()> {
    let mut res = HashSet::new();
    if !attacker_mode && fut_time < solution.statistics.makespan {
        for (name, path) in solution.schedule.iter() {
            if path[fut_time] == coord.as_time(fut_time)
                && announcements.schedule[name][curr_time] > fut_time + 1
            {
                res.insert(path[fut_time + 1].into());
                return Ok(res);
            }
        }
    }
    res = g.neighbors(coord.as_time(1)).map(|tc| tc.into()).collect();
    if fut_time < solution.statistics.makespan {
        for (name, path) in solution.schedule.iter() {
            if announcements.schedule[name][curr_time] > fut_time + 1 {
                res.remove(&path[fut_time + 1].into());
            }
        }
        if !res.contains(&coord) {
            for (_, path) in solution.schedule.iter() {
                if path[fut_time + 1] == coord.as_time(fut_time + 1) {
                    res.remove(&path[fut_time].into());
                }
            }
        }
    }
    res = res
        .difference(
            &conflicts
                .iter()
                .filter(|&tc| tc.t == fut_time + 1)
                .map(|&tc| tc.into())
                .collect(),
        )
        .cloned()
        .collect();
    if res.len() == 0 {
        conflicts.insert(coord.as_time(fut_time));
        return Err(());
    }
    Ok(res)
}

fn print_board(
    floods: &HashMap<String, HashSet<Coordinate>>,
    attacker_name: &String,
    instance: &MapfInstance,
) {
    let mut board = vec![' '; (instance.map.dimensions.x * instance.map.dimensions.y).into()];
    floods.iter().for_each(|(name, flood)| {
        for coord in flood {
            board[(coord.y * instance.map.dimensions.x + coord.x) as usize] =
                if name == attacker_name { 'A' } else { 'D' }
        }
    });
    instance
        .map
        .obstacles
        .iter()
        .for_each(|coord| board[(coord.y * instance.map.dimensions.x + coord.x) as usize] = 'X');
    let colored_board = board
        .iter()
        .map(|x| cell::Cell::new(x.clone(), 50, 0))
        .collect::<Vec<_>>();
    let format = Format::new(1, 1);
    let mut data = matrix::Matrix::new(instance.map.dimensions.x.into(), colored_board);
    let display = MatrixDisplay::new(&format, &mut data);
    display.print(&mut std::io::stdout(), &style::BordersStyle::None);
}

fn defender_observed(
    floods: &HashMap<String, HashSet<Coordinate>>,
    attacker_name: &String,
) -> HashSet<Coordinate> {
    floods
        .iter()
        .filter(|(name, _)| *name != attacker_name)
        .map(|(_, flood)| {
            let mut observable: HashSet<Coordinate> = HashSet::new();
            for coord in flood {
                for x in coord.x - 1..coord.x + 2 {
                    for y in coord.y - 1..coord.y + 2 {
                        observable.insert(Coordinate { x: x, y: y });
                    }
                }
            }
            observable
        })
        .fold(HashSet::new(), |acc, elt| {
            let union: HashSet<_> = acc.union(&elt).cloned().collect();
            union
        })
}

pub fn run_bold_attempts(
    instance: MapfInstance,
    solution: MapfSolution,
    announcements: Announcements,
    mitigation: bool,
) -> BoldExperimentResult {
    let pb = ProgressBar::new(100u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{eta:>5}->{elapsed:>5}] [{wide_bar}] {pos:>7}/{len:>7}")
            .progress_chars("=> "),
    );
    BoldExperimentResult {
        attempts: instance
            .agents
            .iter()
            .take(10)
            .cartesian_product(instance.map.obstacles.iter().take(10))
            .par_bridge()
            .progress_with(pb)
            .map(|(attacker, safe)| {
                run_bold_attempt(
                    &instance,
                    &solution,
                    &announcements,
                    &attacker.name,
                    safe,
                    mitigation,
                )
            })
            .collect(),
    }
}

fn run_bold_attempt(
    instance: &MapfInstance,
    solution: &MapfSolution,
    announcements: &Announcements,
    attacker_name: &String,
    safe: &Coordinate,
    mitigation: bool,
) -> BoldAttemptResult {
    let mut g = build_graph(&instance, &solution, safe);
    let mut attacker_pos = solution.schedule[attacker_name][0];
    let mut deviation: Vec<TimedCoordinate> = Vec::new();
    let mut res = BoldAttemptResult {
        attacker_name: attacker_name.clone(),
        safe: *safe,
        dangerous: false,
        detected: false,
        max_deviated_dist: 0,
        max_inter_observation_time: solution.max_inter_observation_time(&attacker_name),
        min_inter_announcement_time: announcements.min_inter_announcement_time(),
        min_lookahead: announcements.min_lookahead(),
        avg_lookahead: Some(announcements.avg_lookahead()),
    };
    for t in 0..solution.statistics.makespan + 1 {
        deviation.push(attacker_pos);
        let mut dev_sol = solution.clone();
        dev_sol.statistics.makespan = t;
        dev_sol
            .schedule
            .insert(attacker_name.clone(), deviation.clone());
        dev_sol
            .schedule
            .iter_mut()
            .for_each(|(_, sched)| sched.truncate(t + 1));
        if !dev_sol.valid(instance) {
            // caused a collision
            println!("failed: caused collision!");
            res.detected = true;
            return res;
        }
        res.max_deviated_dist = max(
            res.max_deviated_dist,
            attacker_pos.manh_dist(&solution.schedule[attacker_name][t]),
        );
        // update the graph from the announcement
        prune_graph(
            &mut g,
            &instance,
            &solution,
            &attacker_name,
            &announcements,
            t,
            mitigation,
        );
        if mitigation && !g.contains_node(attacker_pos) {
            println!("failed: unexpected observations!");
            res.detected = true;
        }
        if mitigation {
            for agent in &instance.agents {
                if agent.name != *attacker_name {
                    if solution.schedule[&agent.name][t].adj(&solution.schedule[attacker_name][t])
                        && (attacker_pos != solution.schedule[attacker_name][t])
                    {
                        println!(
                            "failed: missed observation! was {:?} but should be {:?}",
                            attacker_pos, solution.schedule[attacker_name][t]
                        );
                        res.detected = true;
                    }
                }
            }
        }
        if Coordinate::from(attacker_pos) == *safe {
            res.dangerous = true;
        }
        if res.dangerous {
            // go to nominal
            let path_to_nominal = astar(
                &g,
                attacker_pos,
                |finish| {
                    (finish.t > t)
                        && (finish.t < announcements.schedule[attacker_name][t])
                        && (finish == solution.schedule[attacker_name][finish.t])
                },
                |_| 1,
                |n| {
                    if n.t < announcements.schedule[attacker_name][t] {
                        n.manh_dist(&solution.schedule[attacker_name][n.t]) / 2 // nominal and deviation head towards each other
                    } else {
                        0
                    }
                },
            );
            attacker_pos = match path_to_nominal {
                Some((_, path)) => path[1],
                None => g
                    .neighbors(attacker_pos)
                    .next()
                    .unwrap_or(attacker_pos.as_time(t)),
            };
        } else {
            // go to safe, meeting known observation constraints
            attacker_pos =
                match next_observed(&instance, &solution, &attacker_name, &announcements, t) {
                    Some(obs) => {
                        // let's check the shortest path to safe, maybe we know a path that still
                        // meets the obs
                        let mut found_known_dev = false;
                        let mut inter = attacker_pos.clone();
                        let mut known_dev_step = attacker_pos.clone();
                        let full_information = announcements
                            .schedule
                            .iter()
                            .map(|(_, sched)| sched[t])
                            .fold(true, |acc, elt| acc && (elt > obs.t));
                        while inter.t < obs.t && !found_known_dev && full_information {
                            let path_to_safe = astar(
                                &g,
                                attacker_pos,
                                |finish| finish.t > inter.t && Coordinate::from(finish) == *safe,
                                |_| 1,
                                |n| safe.manh_dist(&Coordinate::from(n)),
                            );
                            inter = match path_to_safe {
                                Some((_, ref path)) => *path.last().unwrap(),
                                None => break,
                            };
                            let path_from_inter_to_nominal = astar(
                                &g,
                                inter,
                                |finish| {
                                    (finish.t <= obs.t)
                                        && (finish.t < announcements.schedule[attacker_name][t])
                                        && (finish == solution.schedule[attacker_name][finish.t])
                                },
                                |_| 1,
                                |n| {
                                    if n.t < announcements.schedule[attacker_name][t] {
                                        n.manh_dist(&solution.schedule[attacker_name][n.t]) / 2
                                    } else {
                                        0
                                    }
                                },
                            );
                            found_known_dev = match path_from_inter_to_nominal {
                                Some(_) => {
                                    known_dev_step = path_to_safe.unwrap().1[1].clone();
                                    true
                                }
                                None => false,
                            }
                        }
                        if found_known_dev {
                            // can I return early here? let's try it
                            res.dangerous = true;
                            return res;
                        } else {
                            // this just goes to nominal
                            let path_to_nominal = astar(
                                &g,
                                attacker_pos,
                                |finish| {
                                    (finish.t > t)
                                        && (finish.t < announcements.schedule[attacker_name][t])
                                        && (finish == solution.schedule[attacker_name][finish.t])
                                },
                                |_| 1,
                                |n| {
                                    if n.t < announcements.schedule[attacker_name][t] {
                                        n.manh_dist(&solution.schedule[attacker_name][n.t]) / 2
                                    // nominal and deviation head towards each other
                                    } else {
                                        0
                                    }
                                },
                            );
                            match path_to_nominal {
                                Some((_, path)) => path[1],
                                None => g
                                    .neighbors(attacker_pos)
                                    .next()
                                    .unwrap_or(attacker_pos.as_time(t)),
                            }
                        }
                    }
                    None => {
                        let mut found_known_dev = false;
                        let mut inter = attacker_pos.clone();
                        let mut known_dev_step = attacker_pos.clone();
                        let known_horizon = announcements
                            .schedule
                            .iter()
                            .map(|(_, sched)| sched[t])
                            .min()
                            .unwrap();
                        while inter.t < known_horizon && !found_known_dev {
                            let path_to_safe = astar(
                                &g,
                                attacker_pos,
                                |finish| finish.t > inter.t && Coordinate::from(finish) == *safe,
                                |_| 1,
                                |n| safe.manh_dist(&Coordinate::from(n)),
                            );
                            inter = match path_to_safe {
                                Some((_, ref path)) => *path.last().unwrap(),
                                None => break,
                            };
                            let path_from_inter_to_nominal = astar(
                                &g,
                                inter,
                                |finish| {
                                    (finish.t <= known_horizon)
                                        && (finish.t < announcements.schedule[attacker_name][t])
                                        && (finish == solution.schedule[attacker_name][finish.t])
                                },
                                |_| 1,
                                |n| {
                                    if n.t < announcements.schedule[attacker_name][t] {
                                        n.manh_dist(&solution.schedule[attacker_name][n.t]) / 2
                                    } else {
                                        0
                                    }
                                },
                            );
                            found_known_dev = match path_from_inter_to_nominal {
                                Some(_) => {
                                    known_dev_step = path_to_safe.unwrap().1[1].clone();
                                    true
                                }
                                None => false,
                            }
                        }
                        if found_known_dev {
                            res.dangerous = true;
                            return res;
                        }
                        // go to safe
                        let path_to_safe = astar(
                            &g,
                            attacker_pos,
                            |finish| Coordinate::from(finish) == *safe,
                            |_| 1,
                            |n| safe.manh_dist(&Coordinate::from(n)),
                        );
                        match path_to_safe {
                            Some((_, path)) => path[1],
                            None => g
                                .neighbors(attacker_pos)
                                .next()
                                .unwrap_or(attacker_pos.as_time(t)),
                        }
                    }
                };
        }
    }
    res
}

fn next_observed(
    instance: &MapfInstance,
    solution: &MapfSolution,
    attacker_name: &String,
    announcements: &Announcements,
    curr_t: usize,
) -> Option<TimedCoordinate> {
    for t in (curr_t + 1)
        ..min(
            solution.statistics.makespan + 2,
            announcements.schedule[attacker_name][curr_t],
        )
    {
        for agent in &instance.agents {
            if (agent.name != *attacker_name)
                && (t < announcements.schedule[&agent.name][curr_t])
                && solution.schedule[&agent.name][t].adj(&solution.schedule[attacker_name][t])
            {
                return Some(solution.schedule[attacker_name][t]);
            }
        }
    }
    None
}

fn prune_graph(
    g: &mut DiGraphMap<TimedCoordinate, ()>,
    instance: &MapfInstance,
    solution: &MapfSolution,
    attacker_name: &String,
    announcements: &Announcements,
    curr_t: usize,
    mitigation: bool,
) {
    for agent in &instance.agents {
        if agent.name != *attacker_name {
            for t in 1..min(
                solution.statistics.makespan + 2,
                announcements.schedule[&agent.name][curr_t],
            ) {
                let attacker_pos_nominal = solution.schedule[attacker_name][t];
                let prev_occupied = solution.schedule[&agent.name][t - 1];
                let occupied = solution.schedule[&agent.name][t];
                g.remove_edge(occupied.as_time(t - 1), prev_occupied.as_time(t));
                g.remove_node(occupied);
                for n_x in (occupied.x - 1)..(occupied.x + 2) {
                    for n_y in (occupied.y - 1)..(occupied.y + 2) {
                        let observed = TimedCoordinate {
                            x: n_x,
                            y: n_y,
                            t: t,
                        };
                        if mitigation
                            && observed.adj(&occupied)
                            && (observed != attacker_pos_nominal)
                        {
                            g.remove_node(observed);
                        }
                    }
                }
            }
        }
    }
}

fn build_graph(
    instance: &MapfInstance,
    solution: &MapfSolution,
    safe: &Coordinate,
) -> DiGraphMap<TimedCoordinate, ()> {
    let mut g = DiGraphMap::<TimedCoordinate, ()>::default();
    for t in 0..solution.statistics.makespan + 1 {
        for x in 0..instance.map.dimensions.x {
            for y in 0..instance.map.dimensions.y {
                let dest = TimedCoordinate { x: x, y: y, t: t };
                g.add_node(dest);
                if t > 0 {
                    for n_x in (x - 1)..min(instance.map.dimensions.x, x + 2) {
                        for n_y in (y - 1)..min(instance.map.dimensions.y, y + 2) {
                            let source = TimedCoordinate {
                                x: n_x,
                                y: n_y,
                                t: t - 1,
                            };
                            if source.adj(&dest) {
                                g.add_edge(source, dest, ());
                            }
                        }
                    }
                }
            }
        }
    }
    for t in 0..solution.statistics.makespan + 1 {
        for obstacle in &instance.map.obstacles {
            if obstacle != safe {
                g.remove_node(TimedCoordinate {
                    x: obstacle.x,
                    y: obstacle.y,
                    t,
                });
            }
        }
    }
    g
}
