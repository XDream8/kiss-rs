#!/bin/sh -e

current=$(date +%s.%N)
args="$*"
command="${args##*"$1"}"

n="$1"
i=0
while [ ! $i -eq "$n" ]; do
    $command 1>/dev/null
    i=$((i + 1))
done

done=$(date +%s.%N)

echo "$done - $current" | bc
