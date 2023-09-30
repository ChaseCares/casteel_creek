# Casteel Creek

Casteel Creek is a Rust script to generate a simple profile of a given Zillow¹ or Compass URl.

1. Zillow is aggressive about scraping, so what you can do is go to the page you want to save right click and view page source, copy the source into a file and pass the file path as the URL.

The program download the images and create a info text file with some basic information about the location.

```console
# casteel_creek -h
    Usage: casteel_creek [OPTIONS] --name <NAME> --url <URL>

    Options:
    -o, --output <OUTPUT>  Output directory <output>/<name> [default: houses]
    -n, --name <NAME>
        --url <URL>
    -h, --help             Print help
    -V, --version          Print version
```
