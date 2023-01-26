#!/bin/bash

DIR=$(cargo locate-project | jq .root | xargs dirname)
cargo build --release


rm /tmp/*cautious*yaml

echo "10"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-10.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "20"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-20.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "30"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-30.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "40"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-40.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "50"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-50.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "60"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-60.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "70"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-70.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "80"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-80.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "90"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-90.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 

echo "100"
parallel -j 1 --eta --results $DIR/experiments/log-time --timeout 600 \
    time $DIR/target/release/announcenet analyze-attackers -t cautious -a {2} -k {3} -x -m $DIR/experiments/inputs/maps/{1}.yaml \
    -s $DIR/experiments/inputs/plans/{1}.yaml -o /tmp/{1}_cautious_{2}_{3}.yaml \
    :::: $DIR/experiments/instances-short-100.txt \
    ::: kgrouped \
    ::: $(seq -f '%03.f' 1 4 16) 
