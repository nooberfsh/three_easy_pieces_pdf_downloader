extern crate rayon;
extern crate regex;
extern crate reqwest;

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Instant;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use regex::Regex;
use rayon::prelude::*;

const URL: &str = "http://pages.cs.wisc.edu/~remzi/OSTEP/";
const DST: &str = "pdf";

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Reqwest(reqwest::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

struct Pdf {
    no: Option<String>,
    name: String,
}

impl Pdf {
    fn new(no: Option<String>, name: String) -> Self {
        Pdf { no: no, name: name }
    }

    fn full_name(&self) -> String {
        if let Some(ref no) = self.no {
            no.to_string() + "." + &self.name
        } else {
            self.name.clone()
        }
    }

    fn url(&self) -> String {
        URL.to_string() + &self.name
    }
}

fn extract<P: AsRef<Path>>(path: P) -> Result<Vec<Pdf>, Error> {
    let html = fs::File::open(path)?;
    let reader = BufReader::new(html);
    let rg_name = Regex::new(r#"href=(.+pdf)"#).unwrap();
    let rg_no = Regex::new(r#"<small>(\d+)</small>"#).unwrap();
    let mut ret = vec![];

    for line in reader.lines() {
        let line = line?;
        if let Some(pdf) = rg_name.captures(&line) {
            let no = rg_no.captures(&line).map(|no| no[1].to_string());
            ret.push(Pdf::new(no, pdf[1].to_string()));
        }
    }
    Ok(ret)
}

fn download_pdf<P: AsRef<Path>>(pdf: &Pdf, dst: P) -> Result<(), Error> {
    let url = pdf.url();
    let name = pdf.full_name();
    let path = dst.as_ref().to_path_buf().join(name);
    let mut f = fs::File::create(path)?;
    reqwest::get(&url)?.copy_to(&mut f)?;
    Ok(())
}

fn download_html<P: AsRef<Path>>(target: P) -> Result<(), Error> {
    let mut f = fs::File::create(target)?;
    reqwest::get(URL)?.copy_to(&mut f)?;
    Ok(())
}

fn init() -> Result<(), Error> {
    let path: &Path = DST.as_ref();
    if path.exists() {
        if path.is_file() {
            fs::remove_file(path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            //TODO
        }
    }
    Ok(fs::create_dir(path)?)
}

fn main() {
    init().expect("init failed");

    let now = Instant::now();

    let html = PathBuf::new().join(DST).join("data.html");
    println!("Begin to donwload html");
    download_html(&html).expect("download html failed");
    println!("Finish downloading html");

    let pdfs = extract(&html).expect("extect pdf meta failed");
    let succeed_count = AtomicUsize::new(0);
    let failed_count = AtomicUsize::new(0);
    pdfs.par_iter().for_each(|pdf| {
        println!("Begin to download {}", pdf.full_name());
        if let Err(e) = download_pdf(pdf, DST) {
            println!("Download {} failed, reason: {:?}", pdf.full_name(), e);
            failed_count.fetch_add(1, SeqCst);
        } else {
            println!("Download {} success", pdf.full_name());
            succeed_count.fetch_add(1, SeqCst);
        }
    });

    println!(
        "Finishing downoloading {} objects in {:?}, {} success, {} failed",
        pdfs.len(),
        now.elapsed(),
        succeed_count.load(SeqCst),
        failed_count.load(SeqCst)
    );
}
