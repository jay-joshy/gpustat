# `gpustat`

A simple fork of the rust version of the gpustat package. See original repo: https://github.com/AlongWY/gpustat
Made for my own private use case.
Just wanted a simple batteries included continuous output to the terminal of the original.

`-p`, `--show-pid` : Display PID of the process

- `-F`, `--show-fan` : Display GPU fan speed
- `-e`, `--show-codec` : Display encoder and/or decoder utilization
- `-a`, `--show-all` : Display all gpu properties above

## Quick Installation

Install from Cargo:

```
cargo install gpustat_fork
```

## Default display

> [0] | A100-PCIE-40GB | 65'C | 75 % | 33409 / 40536 MB | along(33407M)

- `[0]`: GPUindex (starts from 0) as PCI_BUS_ID
- `A100-PCIE-40GB`: GPU name
- `65'C`: Temperature
- `75 %`: Utilization
- `33409 / 40536 MB`: GPU Memory Usage
- `along(33407M)`: Username of the running processes owner on GPU (and their memory usage)
