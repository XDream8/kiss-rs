#!/bin/sh -e

current=$(date +%s.%N)

i=0
while [ ! $i -eq 1000 ]; do
    $* 1>/dev/null
    i=$((i + 1))
done

done=$(date +%s.%N)

echo "$done - $current" | bc
