use clap::{App, Arg};
use std::fs::File;
use std::io::{self, BufRead};
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::*;

fn main() {
    let matches = App::new("DNS Enumerator")
        .version("0.1.0")
        .author("Author")
        .about("Performs DNS enumeration")
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .value_name("DOMAIN")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("wordlist")
                .short("w")
                .long("wordlist")
                .value_name("WORDLIST")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("lookup_txt")
                .short("t")
                .long("lookup_txt")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name("show_cname")
                .short("c")
                .long("show_cname")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    let domain = matches.value_of("domain").unwrap().to_string();
    let wordlist = matches
        .value_of("wordlist")
        .unwrap_or("default_wordlist.txt")
        .to_string();
    let lookup_txt = matches.is_present("lookup_txt");
    let show_cname = matches.is_present("show_cname");

    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));

    {
        let wordlist = wordlist.clone();
        let domain = domain.clone(); // Clone the domain variable so that it can be used inside the thread
        thread::spawn(move || {
            worker(Arc::clone(&rx), domain, lookup_txt, show_cname);

            // Put the wordlist file reading logic inside the thread
            let file = File::open(&wordlist).unwrap();
            let reader = io::BufReader::new(file);
            for line in reader.lines() {
                let subdomain = format!("{}.{}", line.unwrap(), domain);
                tx.send(subdomain).unwrap();
            }
        });
    }
}

fn worker(
    rx: Arc<Mutex<Receiver<String>>>,
    domain: String,
    lookup_txt: bool,
    show_cname: bool,
) {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

    while let Ok(subdomain) = rx.lock().unwrap().recv() {
        let fqdn = format!("{}.{}", subdomain, domain);

        // Perform the DNS queries and handle the results
        if lookup_txt {
            if let Ok(lookup) = resolver.lookup_txt(&fqdn) {
                // Do something with the TXT records
            }
        }
        if show_cname {
            if let Ok(lookup) = resolver.lookup(&fqdn, RecordType::CNAME) {
                // Do something with the CNAME records
            }
        }
    }
}
