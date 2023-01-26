#!/bin/bash

DIR=$(cargo locate-project | jq .root | xargs dirname)
cargo build --release

test ! $? -eq 0 && exit

for f in $(find $DIR/experiments/outputs -type f -name '*.yaml'); do
    if test ! -s "$f"; then
        echo removing "$f", was empty
        rm "$f"
    fi
done

# for f in $(find $DIR/experiments/ -maxdepth 1 -type f -name '*.png'); do
#     if test ! -s "$f"; then
#         echo removing "$f", was empty
#         rm "$f"
#     fi
# done

find $DIR/experiments/outputs -type f -name '*bold*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-kahead $DIR/experiments/succ-vs-kahead.svg

find $DIR/experiments/outputs -type f -name '*bold*kgrouped*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-kgrouped $DIR/experiments/succ-vs-kgrouped.svg

find $DIR/experiments/outputs -type f -name '*bold*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-max-inter-obs $DIR/experiments/succ-vs-inter-obs.svg

find $DIR/experiments/outputs -type f -name '*no-mitigation*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-kahead $DIR/experiments/succ-vs-kahead_no-mitigation.svg

find $DIR/experiments/outputs -type f -name '*no-mitigation*kgrouped*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-kgrouped $DIR/experiments/succ-vs-kgrouped_no-mitigation.svg


find $DIR/experiments/outputs -type f -name '*no-mitigation*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-max-inter-obs $DIR/experiments/succ-vs-inter-obs_no-mitigation.svg

find $DIR/experiments/outputs -type f -name '*bold*robust*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        succ-vs-robust $DIR/experiments/succ-vs-robust.svg

find $DIR/experiments/outputs -type f -name '*cautious*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-kahead $DIR/experiments/secure-vs-kahead.svg
find $DIR/experiments/outputs -type f -name '*agents10_*cautious*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-kahead $DIR/experiments/secure-vs-kahead_agents10.svg
find $DIR/experiments/outputs -type f -name '*agents50_*cautious*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-kahead $DIR/experiments/secure-vs-kahead_agents50.svg
find $DIR/experiments/outputs -type f -name '*agents90_*cautious*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-kahead $DIR/experiments/secure-vs-kahead_agents90.svg

find $DIR/experiments/outputs -type f -name '*cautious*kgrouped*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-kgrouped $DIR/experiments/secure-vs-kgrouped.svg


find $DIR/experiments/outputs -type f -name '*cautious*kahead*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-max-inter-obs $DIR/experiments/secure-vs-inter-obs.svg

find $DIR/experiments/outputs -type f -name '*cautious*robust*.yaml' | \
    $DIR/target/release/announcenet generate-plots \
        secure-vs-robust $DIR/experiments/secure-vs-robust.svg

parallel --bar 'inkscape --export-type="eps" --export-ps-level="3" {1} 2>/dev/null' ::: $(ls $DIR/experiments/*.svg)
