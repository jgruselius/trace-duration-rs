A simple program I wrote to quickly find the interval between two events in a certain type of log files which use `windows-1252` encoding and the following line format:

```
yyyy-mm-dd HH:MM:SS> log message here
```

Although this can easily be adapted to some other format, or a more generic one.

**Example:**

```shell
# command:
trace-duration-rs -f 'InitializeSystem - start' -t 'TerminateSystem - complete' ./runlog.trc
# output:
"InitializeSystem - start" => "TerminateSystem - complete": +23:59:10 (hh:mm:ss)
```

**Usage:**

```
trace-duration-rs 0.7.0
Joel Gruselius <github.com/jgruselius>
Find the time passed between the (first) occurrence of two strings or patterns in a log file

Usage: trace-duration-rs [OPTIONS] <--from <PATTERN>|--from-last <PATTERN>> <--to <PATTERN>|--to-last <PATTERN>> <FILE>

Arguments:
  <FILE>  The trace file to search

Options:
  -f, --from <PATTERN>       The pattern that defines the start
  -F, --from-last <PATTERN>  The pattern that defines the start (last match)
  -t, --to <PATTERN>         The pattern that defines the end
  -T, --to-last <PATTERN>    The pattern that defines the end (last match)
  -r, --regex                Use regex patterns
  -s, --short                Only print the duration
  -v, --verbose              Print matching lines
  -h, --help                 Print help
  -V, --version              Print version
```