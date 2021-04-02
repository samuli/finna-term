use std::collections::HashMap;
use std::borrow::Cow::{self, Borrowed, Owned};
use std::process::Command;

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
use rustyline::{CompletionType, Config, Context, EditMode, Editor};
use rustyline::config::OutputStreamType;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline_derive::{Helper};

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
pub struct Author {
    pub name: String,
    pub role: Option<String>
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
    #[serde(default)]
    pub year: Option<String>,

    pub primary_authors: Vec<String>,
    pub non_presenter_authors: Vec<Author>,

    #[serde(default)]
    pub images: Vec<String>,
    
    #[serde(flatten)]
    extra: HashMap<String, Value>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordFull {
    #[serde(default)]
    pub full_record: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRaw {
    #[serde(flatten)]
    raw_data: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultsFull {
    #[serde(default)]
    pub records: Vec<RecordFull>,
    #[serde(default)]
    pub result_count: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultsRaw {
    #[serde(default)]
    pub records: Vec<RecordRaw>,
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

#[derive(Helper)]
struct MyHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MyHelper {
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
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

    let authors_to_str = |authors:&Vec<Author>| -> Vec<String> {
        authors.iter().map(|p| p.name.clone()).collect()
    };
    
    let authors = if rec.primary_authors.len() > 0 {
        rec.primary_authors.clone()
    } else if rec.non_presenter_authors.len() > 0 {
        authors_to_str(&rec.non_presenter_authors).clone()
    } else {
        vec![]
    };
               
    let mut building = "";
    if rec.buildings.len() > 0 {
        building = &rec.buildings.first().unwrap().translated;
    }
    let year = match &rec.year {
        Some(year) => format!(" ({})", year.clone()),
        None => "".into()
    };
    println!(
        "{cnt:>3} {title:.len$}{year}  {format} - {format_code}",
        cnt = (cnt + 1).to_string().yellow(),
        title = rec.title.as_ref().unwrap().bold(),
        year = year,
        format = format.yellow(),
        format_code = format_code,
        len = 80
    );
    println!("{fill:>4}{authors}  {building}",
             fill = "",
             building = building.blue(),
             authors = vec2str(&authors, " | ")
    );
}

fn view_results(params: &Params, results: &SearchResults) {
    for (i, rec) in results.records.iter().enumerate() {
        view_result(&rec, i);
    }
    println!(
        "\n{lookfor} ({results} {results_label}, page {page}){filters}",
        lookfor = format!("{}", vec2str(&params.lookfor, " ").yellow().bold()),
        results_label = format!("{}", "results"),
        results = results.result_count.to_string(),
        page = params.page,
        filters = if let Some(filters) = &params.filter {
            format!(", filter: {:?}", filters)
        } else {
            "".to_string()
        }
    );
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
        rec_fields(),
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
                    match query_type {
                        RecordQuery::Fields => {
                            let results: SearchResults = response.json().expect("Error parsing results");
                            println!("{}", serde_json::to_string_pretty(&results.records[0]).unwrap());
                        }
                        RecordQuery::FullRecord => {
                            let results: SearchResultsFull = response.json().expect("Error parsing results");
                            let mut data = serde_json::to_string(&results.records[0].full_record).unwrap();

                            // Clean up
                            data = data.replace("\\n", "")
                                .replace("\"", &'"'.to_string()).replace("\\", "");
                            data = data[1..data.len()-1].to_string();

                            // Add line breaks between tags, preserve indentation
                            let regex = Regex::new(r">(?P<indent>\s*)<").unwrap();
                            data = regex.replace_all(&data.to_string(), ">\n$indent<").to_mut().to_string();

                            println!("{}", data);
                        }
                        RecordQuery::RawData => {
                            let results: SearchResultsRaw = response.json().expect("Error parsing results");
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&results.records[0].raw_data).unwrap()
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

fn rec_fields() -> Vec<String> {
    vec!["id".into(),
         "title".into(),
         "formats".into(),
         "buildings".into(),
         "images".into(),
         "primaryAuthors".into(),
         "nonPresenterAuthors".into(),
         "year".into()
    ]
}

fn search(mut params: Params, session: &mut Session) -> Option<SearchResults> {
    let params_copy = params.clone();
    params.field = rec_fields();
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
fn record_action(action: &str, id: &str, record:&Record, session: &mut Session) {
    let open_record = |holdings: bool| {
        let anchor = if holdings { "#tabnav"} else { "" };
        let rec_url = format!("{url}/Record/{id}/Holdings{anchor}",
                          url = session.app_config.site_url,
                          id = id,
                          anchor = anchor);
        if !open::that(rec_url).is_ok() {
            error("Error opening external program");
        }
    };
    
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
        "img" => {
            if let Some(img) = record.images.get(0) {
                let path = format!("https://finna.fi{}", img);
                Command::new("feh")
                    .arg("--auto-zoom")
                    .arg("--fullscreen")                    
                    .arg("--borderless")
                    .arg(path)
                    .spawn().expect("process failed to execute");
            } else {
                println!("No images");
            }
        }
        "finna" => {
            open_record(false);
        }
        "status" => {
            open_record(true);
        }
        _ => {}
    }
}

fn save_history(reader: &Editor<(MyHelper)>) {
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
    println!("p: {:?}", params);
    
    match search(params.clone(), &mut session) {
        Some(res) => {
            results = res;
        }
        None => {}
    }
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();
    
    let helper = MyHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
    };

    let mut reader = Editor::with_config(config);
    //let mut reader = Editor::<()>::new();
    reader.set_helper(Some(helper));
    
    if let Err(_) = reader.load_history("finna_history.txt") {}

    let mut count = 1;
    loop {
        let p = format!("{}> ", count);
        reader.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{}\x1b[0m", p);
        let readline = reader.readline(&p);
        
        match readline {
            Ok(line) => {
                count = count+1;
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
                                        record_action(cmd, &id, rec, &mut session);
                                    }
                                    None => {
                                        error("Invalid record number");
                                    }
                                },
                                Err(_e) => {
                                    println!("Invalid record index");
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
                                "img" => {
                                    //let imgs = Vec::<String>::new();
                                    let mut imgs:Vec<String> = results.records.iter().map(|rec| rec.images.iter().map(|img| img.clone()).collect()).collect();
                                    imgs = imgs.iter().map(|img| format!("https://finna.fi{}", img)).collect();
                                    if imgs.len() > 0 {
                                        let mut cmd = Command::new("feh");
                                        cmd.arg("--auto-zoom")
                                            .arg("--fullscreen")                    
                                            .arg("--borderless");
                                        for img in imgs {
                                            cmd.arg(img);
                                        }
                                        cmd.spawn().expect("process failed to execute");
                                    } else {
                                        println!("No images");
                                    }
                                }
                                _ => {
                                    error("Unknown command");
                                }
                            }
                        }
                    }
                } else {
                    // Prefix with whitespace to preserve first argument
                    params = Params::from_iter(format!(" {}", line.trim()).split(" "));
                    match search(params.clone(), &mut session) {
                        Some(res) => {
                            results = res;
                        }
                        None => {}
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
