use decorum::N64;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Hash, Eq, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: u16,
    pub y: u16,
}

impl From<TimedCoordinate> for Coordinate {
    fn from(tc: TimedCoordinate) -> Coordinate {
        Coordinate { x: tc.x, y: tc.y }
    }
}

impl Coordinate {
    pub fn adj(&self, other: &Self) -> bool {
        self.manh_dist(other) <= 1
    }

    pub fn manh_dist(&self, other: &Self) -> usize {
        (((self.x as i32) - (other.x as i32)).abs() + ((self.y as i32) - (other.y as i32)).abs())
            as usize
    }
    pub fn as_time(&self, t: usize) -> TimedCoordinate {
        TimedCoordinate {
            x: self.x,
            y: self.y,
            t: t,
        }
    }
}

#[derive(Debug, Hash, Eq, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedCoordinate {
    pub x: u16,
    pub y: u16,
    pub t: usize,
}

impl Ord for TimedCoordinate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.t.cmp(&other.t)
    }
}

impl PartialOrd for TimedCoordinate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TimedCoordinate {
    pub fn adj(&self, other: &Self) -> bool {
        Coordinate::from(*self).adj(&Coordinate::from(*other))
    }
    pub fn manh_dist(&self, other: &Self) -> usize {
        Coordinate::from(*self).manh_dist(&Coordinate::from(*other))
    }
    pub fn as_time(&self, new_time: usize) -> TimedCoordinate {
        TimedCoordinate {
            x: self.x,
            y: self.y,
            t: new_time,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub goal: Coordinate,
    pub name: String,
    pub start: Coordinate,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MapfInstance {
    pub agents: Vec<Agent>,
    pub map: Map,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub dimensions: Coordinate,
    pub obstacles: HashSet<Coordinate>,
}

#[allow(non_snake_case)] // inherit non_snake_case names from libMultiRobotPlanning
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub cost: u32,
    pub makespan: usize,
    pub runtime: f64,
    pub highLevelExpanded: u32,
    pub lowLevelExpanded: u32,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct MapfSolution {
    pub statistics: Statistics,
    pub schedule: HashMap<String, Vec<TimedCoordinate>>,
}

impl MapfSolution {
    pub fn max_inter_observation_time(&self, attacker_name: &String) -> usize {
        let mut iot = 1;
        let mut miot = 1;
        for t in 0..self.statistics.makespan + 1 {
            let mut observation_made = false;
            for agent_name in self.schedule.keys() {
                if (*agent_name != *attacker_name)
                    && self.schedule[agent_name][t].adj(&self.schedule[attacker_name][t])
                {
                    observation_made = true;
                    break;
                }
            }
            if observation_made {
                iot = 1;
            } else {
                iot += 1;
            }
            miot = max(miot, iot);
        }
        miot
    }
    pub fn valid(&self, instance: &MapfInstance) -> bool {
        let mut t: usize = 0;
        loop {
            let pos: HashSet<Coordinate> = self
                .schedule
                .iter()
                .map(|(_, sched)| sched[t].into())
                .collect();
            if pos.len() != instance.agents.len() {
                return false;
            } // vertex conflict
            if pos
                .iter()
                .filter(|coord| {
                    coord.x >= instance.map.dimensions.x || coord.y >= instance.map.dimensions.y
                })
                .count()
                > 0
            {
                return false;
            } // off the map or obstacle collision
            if t == self.statistics.makespan {
                break;
            }
            for a in &instance.agents {
                if self.schedule[&a.name][t].manh_dist(&self.schedule[&a.name][t + 1]) > 1 {
                    return false; // dynamics constraint
                }
                for b in &instance.agents {
                    if a != b
                        && self.schedule[&a.name][t] == self.schedule[&b.name][t + 1]
                        && self.schedule[&b.name][t] == self.schedule[&a.name][t + 1]
                    {
                        return false; // edge conflict
                    }
                }
            }
            t += 1;
        }
        true
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Announcements {
    pub schedule: HashMap<String, Vec<usize>>,
}

impl Announcements {
    pub fn min_inter_announcement_time(&self) -> usize {
        let mut iat = 1;
        let mut miat = self.schedule.values().next().unwrap().len();
        for t in 1..self.schedule.values().next().unwrap().len() {
            let mut announcement_made = false;
            for agent_name in self.schedule.keys() {
                if self.schedule[agent_name][t] != self.schedule[agent_name][t - 1] {
                    announcement_made = true;
                    break;
                }
            }
            if announcement_made {
                miat = min(miat, iat);
                if miat == 1 {
                    break;
                }
                iat = 1;
            } else {
                iat += 1;
            }
        }
        miat
    }
    pub fn min_lookahead(&self) -> usize {
        let mut mlahead = self.schedule.values().next().unwrap().len();
        for t in 0..self.schedule.values().next().unwrap().len() {
            for agent_name in self.schedule.keys() {
                mlahead = min(mlahead, self.schedule[agent_name][t] - t - 1);
            }
        }
        mlahead
    }
    pub fn avg_lookahead(&self) -> N64 {
        let mut laheads: Vec<usize> = Vec::new();
        for t in 0..self.schedule.values().next().unwrap().len() {
            for agent_name in self.schedule.keys() {
                laheads.push(self.schedule[agent_name][t] - t - 1);
            }
        }
        (laheads.iter().sum::<usize>() as f64 / laheads.len() as f64).into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn timed_coordinate_into_coordinate() {
        for x in 0..10 {
            for y in 0..10 {
                let coord = Coordinate { x: x, y: y };
                for t in 0..10 {
                    let tc = TimedCoordinate { x: x, y: y, t: t };
                    assert_eq!(coord, tc.into());
                }
            }
        }
    }
}
