use crate::{structs::{Config, InjectionPlace, RequestDefaults, DataType}, utils::parse_request};
use clap::{crate_version, App, AppSettings, Arg};
use reqwest::Client;
use std::{collections::HashMap, fs, time::Duration, io::{self, Write}, error::Error};
use url::Url;

pub fn get_config() -> Result<(Config, RequestDefaults<'static>, isize), Box<dyn Error>> {

    let app = App::new("x8")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version(crate_version!())
        .author("sh1yo <sh1yo@tuta.io>")
        .about("Hidden parameters discovery suite")
        .arg(Arg::with_name("url")
            .short("u")
            .long("url")
            .help("You can add a custom injection point with %s.")
            .takes_value(true)
            .conflicts_with("request")
        )
        .arg(Arg::with_name("request")
            .short("r")
            .long("request")
            .help("The file with the raw http request")
            .takes_value(true)
            .conflicts_with("url")
        )
        .arg(Arg::with_name("proto")
            .long("proto")
            .help("Protocol to use with request file (default is \"https\")")
            .takes_value(true)
            .requires("request")
            .conflicts_with("url")
        )
        .arg(
            Arg::with_name("wordlist")
                .short("w")
                .long("wordlist")
                .help("The file with parameters (leave empty to read from stdin)")
                .default_value("")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("parameter_template")
                .short("P")
                .long("param-template")
                .help("%k - key, %v - value. Example: --param-template 'user[%k]=%v'\nDefault: urlencoded - <%k=%v>, json - <\"%k\":%v>, headers - <%k=%v>")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("joiner")
            .short("j")
            .long("joiner")
            .help("How to join parameter templates. Example: --joiner '&'\nDefault: urlencoded - '&', json - ', ', headers - '; '")
            .takes_value(true),
        )
        .arg(
            Arg::with_name("body")
                .short("b")
                .long("body")
                .help("Example: --body '{\"x\":{%s}}'\nAvailable variables: {{random}}")
                .value_name("body")
                .conflicts_with("request")
        )
        .arg(
            Arg::with_name("data-type")
                .short("t")
                .long("data-type")
                .help("Available: urlencode, json\nCan be detected automatically if --body is specified (default is \"urlencode\")")
                .value_name("data-type")
        )
        .arg(
            Arg::with_name("proxy")
                .short("x")
                .long("proxy")
                .value_name("proxy")
        )
        .arg(
            Arg::with_name("delay")
                .short("d")
                .long("delay")
                .value_name("Delay between requests in milliseconds")
                .default_value("0")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("file")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("output-format")
                .short("O")
                .long("output-format")
                .help("standart, json, url, request")
                .default_value("standart")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("append")
                .long("append")
                .help("Append to the output file instead of overwriting it.")
        )
        .arg(
            Arg::with_name("method")
                .short("X")
                .long("method")
                .value_name("method")
                .default_value("GET")
                .takes_value(true)
                .conflicts_with("request")
        )
        .arg(
            Arg::with_name("headers")
                .short("H")
                .help("Example: -H 'one:one' 'two:two'")
                .takes_value(true)
                .min_values(1)
                .conflicts_with("request")
        )
        .arg(
            Arg::with_name("as-body")
                .long("as-body")
                .help("Send parameters via body.\nBuilt in body types that can be detected automatically: json, urlencode")
        )
        .arg(
            Arg::with_name("headers-discovery")
                .long("headers")
                .help("Switch to header discovery mode.\nForbidden chars would be automatically removed from headers names")
                .conflicts_with("as-body")
                .conflicts_with("param-template")
        )
        .arg(
            Arg::with_name("force")
                .long("force")
                .help("Ignore 'binary data detected', 'the page is too huge', 'param_template lacks variables' error messages")
        )
        .arg(
            Arg::with_name("disable-custom-parameters")
                .long("disable-custom-parameters")
                .help("Do not check automatically parameters like admin=true")
        )
        .arg(
            Arg::with_name("disable-colors")
                .long("disable-colors")
        )
        .arg(
            Arg::with_name("disable-progress-bar")
                .long("disable-progress-bar")
        )
        .arg(
            Arg::with_name("keep-newlines")
                .long("keep-newlines")
                .help("--body 'a\\r\\nb' -> --body 'a{{new_line}}b'.\nWorks with body only.")
            )
        .arg(
            Arg::with_name("replay-once")
                .long("replay-once")
                .help("If replay proxy is specified, send all found parameters within one request.")
                .requires("replay-proxy")
        )
        .arg(
            Arg::with_name("replay-proxy")
                .takes_value(true)
                .long("replay-proxy")
                .help("Request target with every found parameter via replay proxy at the end.")
        )
        .arg(
            Arg::with_name("custom-parameters")
                .long("custom-parameters")
                .help("Check these parameters with non-random values like true/false yes/no\n(default is \"admin bot captcha debug disable encryption env show sso test waf\")")
                .takes_value(true)
                .min_values(1)
                .conflicts_with("disable-custom-parameters")
        )
        .arg(
            Arg::with_name("custom-values")
                .long("custom-values")
                .help("Check custom parameters with these values (default is \"1 0 false off null true yes no\")")
                .takes_value(true)
                .min_values(1)
                .conflicts_with("disable-custom-parameters")
        )
        .arg(
            Arg::with_name("follow-redirects")
                .long("follow-redirects")
                .short("L")
                .help("Follow redirections")
        )
        .arg(
            Arg::with_name("encode")
                .long("encode")
                .help("Encodes query or body before a request, i.e & -> %26, = -> %3D\nList of chars to encode: \", `, , <, >, &, #, ;, /, =, %")
        )
        .arg(
            Arg::with_name("strict")
                .long("strict")
                .help("Only report parameters that've changed the different parts of a page")
        )
        .arg(
            Arg::with_name("test")
                .long("test")
                .help("Prints request and response")
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Verbose level 0/1/2")
                .default_value("1")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("save-responses")
                .long("save-responses")
                .help("Save request and response to a directory in case the parameter is found")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("disable-cachebuster")
                .long("disable-cachebuster")
        )
        .arg(
            Arg::with_name("learn_requests_count")
                .long("learn-requests")
                .help("Set the custom number of learning requests.")
                .default_value("9")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("max")
                .short("m")
                .long("max")
                .help("Change the maximum number of parameters.\n(default is 128/192/256 for query, 64/128/196 for headers and 512 for body)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("concurrency")
                .short("c")
                .help("The number of concurrent requests")
                .default_value("1")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("verify")
                .long("verify")
                .help("Verify found parameters one more time.")
        )
        .arg(
            Arg::with_name("reflected-only")
                .long("reflected-only")
                .help("Disable page comparison and search for reflected parameters only.")
        );

    let args = app.clone().get_matches();

    if args.value_of("url").is_none() && args.value_of("request").is_none() {
        Err("A target was not provided")?;
    }

    let delay = Duration::from_millis(parse_int(&args, "delay") as u64);

    let learn_requests_count = parse_int(&args, "learn_requests_count");
    let concurrency = parse_int(&args, "concurrency");
    let verbose = parse_int(&args, "verbose");

    let request = match args.value_of("request") {
        Some(val) => fs::read_to_string(val)?,
        None => String::new(),
    };

    //parse default request information
    //either via request file or via provided parameters
    let (
        proto,
        port,
        (
            method,
            host,
            path,
            headers,
            body,
            data_type,
            injection_place
        )
    ) = if !request.is_empty() {
        let mut proto = args.value_of("proto").ok_or("--proto wasn't provided")?.to_string();

        if !proto.ends_with("://") {
            proto = proto + "://"
        }

        let port: u16 = if args.value_of("port").is_some() {
            parse_int(&args, "port") as u16
        } else {
            if proto == "https://" {
                443
            } else {
                80
            }
        };

        (
            proto,
            port,
            parse_request(
                &request,
                args.is_present("as-body")
            )?
        )

    } else {

        let mut injection_place = if args.is_present("as-body") {
            InjectionPlace::Body
        } else if args.is_present("headers-discovery") {
            InjectionPlace::Headers
        } else {
            InjectionPlace::Path
        };

        let mut headers: HashMap<&str, String> = HashMap::new();

        if let Some(val) = args.values_of("headers") {
            for header in val {
                let mut k_v = header.split(':');
                let key = match k_v.next() {
                    Some(val) => val,
                    None => Err("Unable to parse headers")?,
                };
                let value = [
                    match k_v.next() {
                        Some(val) => val.trim().to_owned(),
                        None => Err("Unable to parse headers")?,
                    },
                    k_v.map(|x| ":".to_owned() + x).collect(),
                ].concat();

                if value.contains("%s") {
                    injection_place = InjectionPlace::HeaderValue;
                }

                headers.insert(key, value);
            }
        };

        //set default headers if weren't specified by a user.
        if !headers.keys().any(|i| i.contains("User-Agent")) {
            headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/83.0.4103.97 Safari/537.36".to_string());
        }

        if !args.is_present("disable-cachebuster") {
            if !headers.keys().any(|i| i.contains("Accept")) {
                headers.insert("Accept", "*/*, text/{{random}}".to_string());
            }
            if !headers.keys().any(|i| i.contains("Accept-Language")) {
                headers.insert("Accept-Language","en-US, {{random}};q=0.9, *;q=0.5".to_string());
            }
            if !headers.keys().any(|i| i.contains("Accept-Charset")) {
                headers.insert("Accept-Charset","utf-8, iso-8859-1;q=0.5, {{random}};q=0.2, *;q=0.1".to_string());
            }
        }

        //TODO replace with ".parse" or sth like it
        let data_type = match args.value_of("data-type") {
            Some(val) => if val == "json" {
                Some(DataType::Json)
            } else if val == "urlencoded" {
                Some(DataType::Urlencoded)
            } else {
                None
            },
            None => None
        };

        let url = Url::parse(args.value_of("url").unwrap())?;
        (
            url.scheme().to_string(),
            url.port_or_known_default().ok_or("Wrong scheme")?,
            (
                args.value_of("method").unwrap().to_string(),
                url.host_str().ok_or("Host missing")?.to_string(),
                url[url::Position::BeforePath..].to_string(), //we need not only the path but query as well
                headers,
                args.value_of("body").unwrap_or("").to_string(),
                data_type,
                injection_place
            )
        )
    };

    let max: isize = if args.is_present("max") {
        parse_int(&args, "max") as isize
    } else {
        match injection_place {
            InjectionPlace::Body => -512,
            InjectionPlace::Path => -128,
            InjectionPlace::Headers => -64,
            InjectionPlace::HeaderValue => -64,
        }
    };

    let body = if args.is_present("keep-newlines") {
        body.replace("\\n", "\n").replace("\\r", "\r")
    } else {
        body
    };

    let url = format!("{}{}:{}{}", proto, host, port, path);

    let custom_keys: Vec<String> = match args.values_of("custom-parameters") {
        Some(val) => {
            val.map(|x| x.to_string()).collect()
        }
        None =>["admin", "bot", "captcha", "debug", "disable", "encryption", "env", "show", "sso", "test", "waf"]
            .iter()
            .map(|x| x.to_string())
            .collect()
    };

    let custom_values: Vec<String> = match args.values_of("custom-values") {
        Some(val) => {
            val.map(|x| x.to_string()).collect()
        }
        None => ["1", "0", "false", "off", "null", "true", "yes", "no"]
            .iter()
            .map(|x| x.to_string())
            .collect()
    };

    let mut custom_parameters: HashMap<String, Vec<String>> = HashMap::with_capacity(custom_keys.len());
    for key in custom_keys.iter() {
        let mut values: Vec<String> = Vec::with_capacity(custom_values.len());
        for value in custom_values.iter() {
            values.push(value.to_string());
        }
        custom_parameters.insert(key.to_string(), values);
    }

    if args.is_present("disable-colors") {
        colored::control::set_override(false);
    }

    //TODO maybe replace empty with None
    let config = Config {
        url: args.value_of("url").unwrap_or(&url).to_string(),
        wordlist: args.value_of("wordlist").unwrap_or("").to_string(),
        custom_parameters,
        proxy: args.value_of("proxy").unwrap_or("").to_string(),
        replay_proxy: args.value_of("replay-proxy").unwrap_or("").to_string(),
        replay_once: args.is_present("replay-once"),
        output_file: args.value_of("output").unwrap_or("").to_string(),
        save_responses: args.value_of("save-responses").unwrap_or("").to_string(),
        output_format: args.value_of("output-format").unwrap_or("").to_string(),
        append: args.is_present("append"),
        force: args.is_present("force"),
        strict: args.is_present("strict"),
        disable_custom_parameters: args.is_present("disable-custom-parameters"),
        disable_progress_bar: args.is_present("disable-progress-bar"),
        follow_redirects: args.is_present("follow-redirects"),
        test: args.is_present("test"),
        verbose,
        learn_requests_count,
        concurrency,
        verify: args.is_present("verify"),
        reflected_only: args.is_present("reflected-only")
    };

    //build client
    let mut client = Client::builder()
        //.resolve("localhost", "127.0.0.1".parse().unwrap())
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(60))
        .http1_title_case_headers()
        .cookie_store(true)
        .use_rustls_tls();

    if !config.proxy.is_empty() {
        client = client.proxy(reqwest::Proxy::all(&config.proxy).unwrap());
    }
    if !config.follow_redirects {
        client = client.redirect(reqwest::redirect::Policy::none());
    }

    let client = client.build()?;

    let request_defaults = RequestDefaults::new(
        &method,
        &url,
        headers,
        delay,
        client,
        args.value_of("parameter_template"),
        args.value_of("joiner"),
        args.is_present("encode"),
        data_type,
        injection_place,
        &body,
    )?;


    Ok((config, request_defaults, max))
}

//TODO remove this func and just use ?
fn parse_int(args: &clap::ArgMatches, value: &str) -> usize {
    match args.value_of(value).unwrap().parse() {
        Ok(val) => val,
        Err(err) => {
            writeln!(io::stderr(), "Unable to parse '{}' value: {}", value, err).ok();
            std::process::exit(1);
        }
    }
}