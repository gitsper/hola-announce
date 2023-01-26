#!/bin/bash

DIR=$(cargo locate-project | jq .root | xargs dirname)
cargo build --release

mkdir -p $DIR/experiments/outputs

for f in $(find $DIR/experiments/outputs -type f -name '*.yaml'); do
    if test ! -s "$f"; then
        echo removing "$f", was empty
        rm "$f"
    fi
done

# parallel -j 1 --eta --results $DIR/experiments/log --timeout 600 \
#     $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
#     -s $DIR/experiments/inputs/plans/{1}.yaml -o $DIR/experiments/outputs/{1}_cautious_{2}_{3}.yaml \
#     :::: $DIR/experiments/instances.txt \
#     ::: kahead kgrouped \
#     ::: $(seq -f '%03.f' 1 4 41) 

parallel -j 1 --eta --results $DIR/experiments/log --timeout 600 \
    $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o $DIR/experiments/outputs/{1}_cautious_{2}.yaml \
    :::: $DIR/experiments/instances.txt \
    ::: robust \

# $DIR/target/release/announcenet analyze-attackers -t cautious -a kahead -k 005 -x -m $DIR/experiments/inputs/maps/map_32by32_obst204_agents10_ex25.yaml \
#     -s $DIR/experiments/inputs/plans/map_32by32_obst204_agents10_ex25.yaml -o $DIR/experiments/outputs/map_32by32_obst204_agents10_ex25_cautious_kahead_001.yaml
