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

parallel -j 1 --eta --results $DIR/experiments/log \
    $DIR/target/release/announcenet analyze-attackers -t bold -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o $DIR/experiments/outputs/{1}_bold_{2}_{3}.yaml \
    :::: $DIR/experiments/instances.txt \
    ::: kahead kgrouped \
    ::: $(seq -f '%03.f' 1 4 41) 

parallel -j 1 --eta --results $DIR/experiments/log \
    $DIR/target/release/announcenet analyze-attackers -t bold -a {2} -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o $DIR/experiments/outputs/{1}_bold_{2}.yaml \
    :::: $DIR/experiments/instances.txt \
    ::: robust \
