use std::collections::HashMap;
use std::result::Result;

extern crate serde;
extern crate colored;
extern crate regex;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_qs;
extern crate confy;

use colored::*;
use regex::Regex;
use serde_json::Value;
use structopt::StructOpt;
extern crate open;
extern crate rustyline;
use rustyline::error::ReadlineError;
use rustyline::Editor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedString {
    pub value: String,
    pub translated: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OnlineUrl {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source: Vec<TranslatedString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub formats: Vec<TranslatedString>,
    #[serde(default)]
    pub buildings: Vec<TranslatedString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Vec<String>>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
    // #[serde(default)]
    // pub links: Vec<OnlineUrl>
}

impl Record {
    pub fn get_authors(&self) -> Option<Vec<String>> {
        match self.extra.get("authors") {
            None => None,
            Some(authors) => {
                let res: Result<(Value), serde_json::error::Error> =
                    serde_json::from_str(&authors.to_string());
                match res {
                    Ok(authors) => match authors.get("primary") {
                        Some(primary) => {
                            if primary.is_object() {
                                match primary.as_object() {
                                    Some(primary) => {
                                        return Some(
                                            primary.iter().map(|(name, _)| name.clone()).collect(),
                                        );
                                    }
                                    None => {}
                                }
                            }
                        }
                        None => {}
                    },
                    _ => {}
                }
                return None;
            }
        }
    }
    pub fn get_other_authors(&self) -> Option<Vec<String>> {
        match self.extra.get("nonPresenterAuthors") {
            None => None,
            Some(authors) => {
                let res: Result<(Value), serde_json::error::Error> =
                    serde_json::from_str(&authors.to_string());
                match res {
                    Ok(authors) => {
                        if authors.is_object() {
                            match authors.as_object() {
                                Some(authors) => {
                                    return Some(
                                        authors.iter().map(|(author, _)| author.clone()).collect(),
                                    );
                                }
                                None => {}
                            }
                        }
                    }
                    _ => {}
                }
                return None;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    api_url: String,
    site_url: String,
}
impl ::std::default::Default for AppConfig {
    fn default() -> Self { Self {
        api_url: "https://api.finna.fi/api/v1".into(),
        site_url: "https://finna.fi".into(),
    }}
}

pub struct Session {
    pub last_search: Option<String>,
    pub app_config: AppConfig
}
impl Default for Session {
    fn default() -> Self {
        Self {
            last_search: None,
            app_config: AppConfig::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    #[serde(default)]
    pub records: Vec<Record>,
    #[serde(default)]
    pub result_count: i32,
}

#[derive(StructOpt, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Params {
    #[structopt(default_value = "")]
    lookfor: Vec<String>,

    #[structopt(long = "type", short = "t", default_value = "AllFields")]
    r#type: String,

    #[structopt(long, short)]
    filter: Option<Vec<String>>,
    #[structopt(long, short, default_value = "20")]
    limit: i32,
    #[structopt(long, short, default_value = "1")]
    page: i32,
    #[structopt(long, default_value = "fi")]
    lng: String,

    #[structopt(long, default_value = "[]")]
    field: Vec<String>,
}

#[derive(StructOpt, Debug, Clone, Serialize, Deserialize)]
struct RecordParams {
    #[structopt(default_value = "")]
    id: Vec<String>,
    field: Vec<String>,
}

fn vec2str(vec: &Vec<String>, delimiter: &str) -> String {
    if vec.len() == 1 {
        return (vec[0].clone().trim()).to_string();
    } else {
        let tot = vec.len();
        vec.iter()
            .enumerate()
            .fold(String::new(), |acc, (i, arg)| {
                let mut res = acc + &arg;
                if i < tot - 1 {
                    res = res + delimiter;
                }
                res.to_string()
            })
            .trim()
            .to_string()
    }
}

fn view_result(rec: &Record, cnt: usize) {
    let (format, format_code) = match rec.formats.clone().pop() {
        Some(format) => (format.translated, format.value),
        None => ("?".to_string(), "?".to_string()),
    };

    let authors = match rec.get_authors() {
        Some(authors) => { vec2str(&authors, " | ")}
        None => { "".to_string() }
    };
    let other_authors = match rec.get_other_authors() {
        Some(authors) => { vec2str(&authors, " | ")}
        None => { "".to_string() }
    };

    let mut building = "";
    if rec.buildings.len() > 0 {
        building = &rec.buildings.first().unwrap().translated;
    }

    println!(
        "{cnt:>3} {title:.len$} {authors}{other_authors}",
        cnt = (cnt + 1).to_string().yellow(),
        title = rec.title.as_ref().unwrap(),
        authors = authors,
        other_authors = other_authors,
        len = 80
    );
    println!(
        "{fill:>3} {format} {format_code} {id:.len$} {building}",
        fill = "",
        len = 30,
        format = format.green().bold(),
        format_code = format_code.green(),
        id = rec.id.as_ref().unwrap().cyan(),
        building = building.blue()
    );
}

fn view_results(params: &Params, results: &SearchResults) {
    println!(
        "{lookfor} ({results} {results_label}, page {page})",
        //             lookfor_label = format!("{}", "Search".yellow().bold()),
        lookfor = format!("{}", vec2str(&params.lookfor, " ").yellow().bold()),
        results_label = format!("{}", "results"),
        results = results.result_count.to_string(),
        page = params.page
    );
    println!("");

    for (i, rec) in results.records.iter().enumerate() {
        view_result(&rec, i);
    }
}

enum RecordQuery {
    Fields,
    RawData,
    FullRecord,
}
fn record_view_raw(id: &str, session: &mut Session) {
    record(
        RecordQuery::RawData,
        &id,
        vec!["rawData".to_string()],
        session,
    )
}
fn record_view_full_record(id: &str, session: &mut Session) {
    record(
        RecordQuery::FullRecord,
        &id,
        vec!["fullRecord".to_string()],
        session,
    )
}

fn record_view(id: &str, session: &mut Session) {
    record(
        RecordQuery::Fields,
        &id,
        vec![
            "id".to_string(),
            "title".to_string(),
            "formats".to_string(),
            "buildings".to_string(),
        ],
        session,
    )
}

fn call_api(url: &str, _session: &mut Session) -> Option<reqwest::Response> {
    debug(&url);
    match reqwest::get(url) {
        Ok(response) => {
            if response.status().is_success() {
                return Some(response);
            }
        }
        _ => {
            return None;
        }
    }
    return None;
}
fn debug(msg: &str) {
    println!("\n{}\n", msg.dimmed());
}
fn error(msg: &str) {
    println!("\n{}\n", msg.red().bold());
}

fn record(query_type: RecordQuery, id: &str, fields: Vec<String>, session: &mut Session) {
    let params = RecordParams {
        id: vec![id.to_string()],
        field: fields,
    };
    let query = serde_qs::to_string(&params);
    match query {
        Ok(query) => {
            let url = session.app_config.api_url.to_owned() + &"/record?" + &query;
            match call_api(&url, session) {
                Some(mut response) => {
                    let results: SearchResults = response.json().expect("Error parsing results");
                    match query_type {
                        RecordQuery::Fields => {
                            println!("{:?}", results.records[0]);
                            //view_result(&results.records[0]);
                        }
                        RecordQuery::FullRecord => {
                            let mut data = serde_json::to_string(&results.records[0].extra["fullRecord"]).unwrap();

                            // Clean up
                            data = data.replace("\\n", "")
                                .replace("\"", &'"'.to_string()).replace("\\", "");
                            data = data[1..data.len()-1].to_string();

                            // Add line breaks between tags, preserve indentation
                            let regex = Regex::new(r">(?P<indent>\s*)<").unwrap();
                            data = regex.replace_all(&data.to_string(), ">\n$indent<").to_mut().to_string();

                            println!("{}", data);
                        }
                        _ => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&results.records[0].extra).unwrap()
                            );
                        }
                    }
                }
                _ => {
                    error("Network error");
                }
            }
        }
        _ => {
            error("Invalid url");
        }
    }
}

fn search(mut params: Params, session: &mut Session) -> Option<SearchResults> {
    let params_copy = params.clone();
    params.field = vec!["id".into(), "title".into(), "formats".into(), "buildings".into(), "primaryAuthors".into(), "nonPresenterAuthors".into()];
    let lookfor = &vec2str(&params.lookfor, " ");
    params.lookfor = vec![];

    let query = serde_qs::to_string(&params);
    match query {
        Ok(mut query) => {
            query = query + "&lookfor=" + &lookfor;
            let url = session.app_config.api_url.to_owned() + &"/search?" + &query;
            session.last_search = Some(query);
            match call_api(&url, session) {
                Some(mut response) => {
                    let results: SearchResults = response.json().expect("Error parsing results");
                    view_results(&params_copy, &results);
                    return Some(results);
                }
                _ => {
                    error("Network error");
                }
            }
        }
        _ => {
            error("Invalid url");
        }
    }
    None
}
fn record_action(action: &str, id: &str, session: &mut Session) {
    match action {
        "s" => {
            record_view(id, session);
        }
        "raw" => {
            record_view_raw(id, session);
        }
        "full" => {
            record_view_full_record(id, session);
        }
        "finna" => {
            let site_url = format!("{url}/Record/{id}", url = session.app_config.site_url, id = id);
            if !open::that(site_url).is_ok() {
                error("Error opening external program");
            }
        }
        _ => {}
    }
}

fn save_history(reader: &Editor<()>) {
    reader.save_history("finna_history.txt").unwrap();
}
fn main() {
    let app_config: AppConfig = confy::load("finna-term").unwrap_or_default();
    println!("{:#?}", app_config);

    let mut session = Session::default();
    session.app_config = app_config;
    
    let mut results = SearchResults {
        result_count: 0,
        records: [].to_vec(),
    };

    let mut params = Params::from_args();
    match search(params.clone(), &mut session) {
        Some(res) => {
            results = res;
        }
        None => {}
    }

    let mut reader = Editor::<()>::new();
    if let Err(_) = reader.load_history("finna_history.txt") {}

    loop {
        let readline = reader.readline("> ");

        match readline {
            Ok(line) => {
                reader.add_history_entry(&line);

                let regex = Regex::new(r"^:([a-z]+)( ([\w\.]+))?$").unwrap();

                if regex.is_match(line.as_ref()) {
                    // Parse colon command
                    let cap = regex.captures(line.as_ref()).unwrap();
                    let cmd = &cap[1].trim();

                    match cap.get(3) {
                        // command with argument
                        Some(id) => {
                            let rec_id = id.as_str();
                            match rec_id.parse::<usize>() {
                                Ok(num) => match results.records.get(num - 1) {
                                    Some(rec) => {
                                        let id = rec.id.as_ref().unwrap().to_string();
                                        record_action(cmd, &id, &mut session);
                                    }
                                    None => {
                                        error("Invalid record number");
                                    }
                                },
                                Err(_e) => {
                                    record_action(cmd, rec_id, &mut session);
                                }
                            }
                        }
                        None => {
                            // command without argument
                            match cmd.as_ref() {
                                "q" => {
                                    save_history(&reader);
                                    break;
                                }
                                "n" => {
                                    params.page = params.page + 1;
                                    match search(params.clone(), &mut session) {
                                        Some(res) => {
                                            results = res;
                                        }
                                        None => {}
                                    }
                                }
                                "r" => {
                                    search(params.clone(), &mut session);
                                }
                                "finna" => {
                                    match &session.last_search {
                                        Some(query) => {
                                            let site_url = format!(
                                                "{url}/Search/Results?{query}",
                                                url = session.app_config.site_url,
                                                query = query
                                            );
                                            if !open::that(site_url).is_ok() {
                                                error("Error opening external program");
                                            }
                                        }
                                        _ => {}
                                    };
                                }
                                _ => {
                                    error("Unknown command");
                                }
                            }
                        }
                    }
                } else {
                    // Parse search query
                    let mut args = vec![""];
                    let mut input_args = line.split(" ").collect();
                    args.append(&mut input_args);
                    match Params::from_iter_safe(args) {
                        Ok(params) => match search(params.clone(), &mut session) {
                            Some(res) => {
                                results = res;
                            }
                            None => {}
                        },
                        Err(_e) => {
                            error("Could not parse input");
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                save_history(&reader);
                break;
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}
