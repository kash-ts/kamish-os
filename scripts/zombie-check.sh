#!/usr/bin/env bash
# Что делает старый контейнер (Up 3 hours)?
docker top firmware-builder-run-3580641f5bf6 -eo pid,etime,pcpu,comm,args 2>/dev/null | grep -v -E 'COMMAND|ps |grep' | head -15
