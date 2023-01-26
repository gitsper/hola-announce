use clap::{App, Arg, ArgMatches, SubCommand};

pub fn parse_opts() -> ArgMatches<'static> {
    App::new("AnnounceNet")
        .version("0.1.0")
        .author("anonymous")
        .subcommand(SubCommand::with_name("compute-secure-announcements"))
        .subcommand(
            SubCommand::with_name("generate-plots")
                .about("read list of .yaml output files from stdin and generate plots")
                .arg(
                    Arg::with_name("output")
                        .required(true)
                        .index(2)
                        .help("path to output file"),
                )
                .arg(
                    Arg::with_name("plot")
                        .required(true)
                        .index(1)
                        .possible_values(&[
                            "succ-vs-kahead",
                            "succ-vs-max-inter-obs",
                            "succ-vs-kgrouped",
                            "succ-vs-robust",
                            "secure-vs-kahead",
                            "secure-vs-max-inter-obs",
                            "secure-vs-kgrouped",
                            "secure-vs-robust",
                        ]),
                )
                .arg(
                    Arg::with_name("force")
                        .short("f")
                        .help("overwrite existing plots"),
                ),
        ).subcommand(
            SubCommand::with_name("analyze-attackers")
                .arg(
                    Arg::with_name("type")
                        .required(true)
                        .takes_value(true)
                        .short("t")
                        .long("type")
                        .help("the attacker type")
                        .possible_values(&[
                            "bold",
                            "cautious",
                            ]),
                )
                .arg(
                    Arg::with_name("mapf-instance")
                        .required(true)
                        .takes_value(true)
                        .short("m")
                        .long("mapf-instance")
                        .display_order(0)
                        .help("path to instance YAML"),
                )
                .arg(
                    Arg::with_name("mapf-solution")
                        .required(true)
                        .takes_value(true)
                        .short("s")
                        .long("mapf-solution")
                        .display_order(1)
                        .help("path to solution YAML corres. to <mapf-instance>"),
                )
                .arg(
                    Arg::with_name("announcement-strategy")
                        .required(true)
                        .takes_value(true)
                        .short("a")
                        .long("announcement-strategy")
                        .possible_values(&["kahead", "kgrouped", "robust", "custom"])
                        .display_order(2)
                        .help("announcement strategy to use"),
                )
                .arg(
                    Arg::with_name("output")
                        .required(true)
                        .takes_value(true)
                        .short("o")
                        .long("output")
                        .display_order(3)
                        .help("path to output file"),
                )
                .arg(
                    Arg::with_name("lookahead")
                        .takes_value(true)
                        .short("k")
                        .long("lookahead")
                        .required_ifs(&[("announcement-strategy", "kahead"),("announcement_strategy","kgrouped")])
                        .help("if using kahead strategy; fixed lookahead. if using kgrouped strategy; k grouping."),
                )
                .arg(
                    Arg::with_name("custom-announcements")
                        .takes_value(true)
                        .short("c")
                        .long("custom-announcements")
                        .required_if("announcement-strategy", "custom")
                        .help("if using custom strategy, path to custom lookaheads YAML"),
                )
                .arg(
                    Arg::with_name("skip-large")
                        .short("x")
                        .long("skip-large")
                        .help("if provided lookahead is larger than makespan, do nothing"),
                )
                .arg(
                    Arg::with_name("no-mitigation")
                    .short("n")
                    .long("no-mitigation")
                    .help("disable detections due to incorrect co-observations"),
                )
        ).get_matches()
}
