use crate::{
    structs::{Config, Stable, RequestDefaults, Request, FoundParameter},
};
use colored::*;
use reqwest::Client;
use std::{
    io::{self, Write}, error::Error,
};

const MAX_PAGE_SIZE: usize = 25 * 1024 * 1024; //25MB usually

///makes first requests and checks page behavior
pub async fn empty_reqs<'a>(
    config: &Config,
    request_defaults: &'a RequestDefaults<'a>,
    count: usize,
    max: usize,
) -> Result<(Vec<String>, Stable), Box<dyn Error>> {
    let mut stable = Stable {
        body: true,
        reflections: true,
    };
    let mut diffs: Vec<String> = Vec::new();

    for i in 0..count {
        let response =
            Request::new_random(request_defaults, max)
                .send()
                .await?;

        //progress bar
        if config.verbose > 0 && !config.disable_progress_bar {
            write!(
                io::stdout(),
                "{} {}/{}       \r",
                &"-> ".bright_green(),
                i,
                count
            ).ok();
            io::stdout().flush().unwrap_or(());
        }

        //do not check pages >25MB because usually its just a binary file or sth
        if response.body.len() > MAX_PAGE_SIZE && !config.force {
            Err("The page is too huge")?;
        }

        if !response.reflected_parameters.is_empty() {
            stable.reflections = false;
        }

        let (is_code_diff, mut new_diffs) = response.compare(&diffs)?;

        if is_code_diff {
            Err("The page is not stable (code)")?
        }

        diffs.append(&mut new_diffs);
    }

    //check the last time
    let response =
        Request::new_random(request_defaults, max)
            .send()
            .await?;

    //in case the page is still different from other random ones - the body isn't stable
    if !response.compare(&diffs)?.1.is_empty() {
        if config.verbose > 0 {
            writeln!(
                io::stdout(),
                "The page is not stable (body)",
            ).ok();
        }
        stable.body = false;
    }

    Ok((diffs, stable))
}

pub async fn verify<'a>(
    request_defaults: &RequestDefaults<'a>, found_params: &Vec<FoundParameter>, diffs: &Vec<String>, stable: &Stable
) -> Result<Vec<FoundParameter>, Box<dyn Error>> {
    //TODO maybe implement sth like similar patters? At least for reflected parameters
    //struct Pattern {kind: PatterKind, pattern: String}
    //
    //let mut similar_patters: HashMap<Pattern, Vec<String>> = HashMap::new();
    //
    //it would allow to fold parameters like '_anything1', '_anything2' (all that starts with _)
    //to just one parameter in case they have the same diffs
    //sth like a light version of --strict

    let mut filtered_params = Vec::with_capacity(found_params.len());

    for param in found_params {

        let mut response = Request::new(&request_defaults, vec![param.name.clone()])
                                    .send()
                                    .await?;

        let (is_code_the_same, new_diffs) = response.compare(&diffs)?;
        let mut is_the_body_the_same = true;

        if !new_diffs.is_empty() {
            is_the_body_the_same = false;
        }

        response.fill_reflected_parameters();

        if !is_code_the_same || !(!stable.body || is_the_body_the_same) || !response.reflected_parameters.is_empty() {
            filtered_params.push(param.clone());
        }
    }

    Ok(filtered_params)
}

pub async fn replay<'a>(
    config: &Config, request_defaults: &RequestDefaults<'a>, replay_client: &Client, found_params: &Vec<FoundParameter>
) -> Result<(), Box<dyn Error>> {
     //get cookies
    Request::new(request_defaults, vec![])
        .send_by(replay_client)
        .await?;

    if config.replay_once {
        Request::new(request_defaults, found_params.iter().map(|x| x.name.to_owned()).collect::<Vec<String>>())
            .send_by(replay_client)
            .await?;
    } else {
        for param in found_params {
            Request::new(request_defaults, vec![param.name.to_string()])
                .send_by(replay_client)
                .await?;
        }
    }

    Ok(())
}