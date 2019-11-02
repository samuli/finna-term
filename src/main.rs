use std::collections::HashMap;

#[macro_use]
extern crate serde;
extern crate serde_derive;
extern crate serde_qs;
extern crate reqwest;
extern crate colored;

use colored::*;
use reqwest::Error;
use serde_json::Value;
use structopt::StructOpt;

extern crate rustyline;
use rustyline::Editor;
use rustyline::error::ReadlineError;

#[derive(Debug, Clone, Deserialize)]
pub struct TranslatedString {
    pub value: String,
    pub translated: String
}

#[derive(Debug, Clone, Deserialize)]
pub struct OnlineUrl {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source: Vec<TranslatedString>
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub title: String,
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
    pub fn get_authors(&self) -> Option<Vec<String>>{
        match self.extra.get("authors") {
            None => { None }
            Some(authors) => {
                let res: Result<(Value), serde_json::error::Error> = serde_json::from_str(&authors.to_string());
                match res {
                    Ok(authors) => {
                        match authors.get("primary") {
                            Some(primary) => {
                                if primary.is_object() {
                                    match primary.as_object() {
                                        Some(primary) => {
                                            return Some(primary.iter().map(|(name, _)| name.clone()).collect());
                                        }
                                        None => {}
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    _ => {}
                }
                return None;
            }
        }
    }
    pub fn get_other_authors(&self) -> Option<Vec<String>>{
        match self.extra.get("nonPresenterAuthors") {
            None => { None }
            Some(authors) => {
                let res: Result<(Value), serde_json::error::Error> = serde_json::from_str(&authors.to_string());
                match res {
                    Ok(authors) => {
                        if authors.is_object() {
                            match authors.as_object() {
                                Some(authors) => {
                                    return Some(authors.iter().map(|(author,_)| author.clone()).collect());
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    #[serde(default)]
    pub records: Vec<Record>,
    pub result_count: i32,
}

#[derive(StructOpt, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Params {
    #[structopt(default_value="")]
    lookfor: Vec<String>,
    #[structopt(long, short)]
    filter: Option<Vec<String>>,
    #[structopt(long, short, default_value="20")]
    limit: i32,
    #[structopt(long, short, default_value="1")]
    page: i32,
    #[structopt(long, default_value="fi")]
    lng: String
}

fn vec2str(vec: &Vec<String>, delimiter: &str) -> String {
    if vec.len() == 1 {
        return (vec[0].clone().trim()).to_string();
    } else {
        let tot = vec.len();
        vec.iter().enumerate().fold(String::new(), |acc, (i, arg)| {
            let mut res = acc + &arg;
            if i < tot-1 {
                res = res + delimiter;
            }
            return (res.trim()).to_string();
        })
    }
}

fn render_record(rec: &Record) {
    let format = match rec.formats.clone().pop() {
        Some(format) => { format.translated }
        None => { "?".to_string() }
    };
    
    let authors = match rec.get_authors() {
        Some(authors) => { vec2str(&authors, " | ")}
        None => { "".to_string() }
    };
    let other_authors = match rec.get_other_authors() {
        Some(authors) => { vec2str(&authors, " | ")}
        None => { "".to_string() }
    };

    //    let buildings = vec2str(&rec.buildings.iter().map(|b| b.translated.clone()).collect(), " > ");
    let mut building = "";
    if rec.buildings.len() > 0 {
        building = &rec.buildings.first().unwrap().translated;
    }
    
    println!("{title} [{authors} {other_authors}] {format} {building}",
             title = rec.title.bold().green(),
             authors = authors,
             other_authors = other_authors,
             format = format.green(),
             building = building.dimmed()
    );
}

fn render_results(params: &Params, results: &SearchResults) {
    println!("{total} results (page {page})",
             total = results.result_count.to_string().dimmed().bold(), page = params.page);
    
    for rec in results.records.iter() {
        render_record(&rec);
    }
}
fn search(mut params: Params) -> Result<(), Error> {
    let lookfor = vec2str(&params.lookfor, " ");
    params.lookfor = vec![];

    let query = serde_qs::to_string(&params);
    match query {
        Ok(mut query) => {
            query = query + "&lookfor=" + &lookfor;
            let url = "https://api.finna.fi/api/v1/search?".to_owned() + &query;
            println!("{}", url.dimmed());
            let mut response = reqwest::get(&url).expect("Error reading from API");
            if response.status().is_success() {
                let results: SearchResults = response.json().expect("Error parsing results");
                render_results(&params, &results);
            } else {
                println!("{}", "Search error".bold().red());
            }
        }
        _ => { println!("{}", "Invalid url".bold().red()); }
    }
    Ok(())
}

fn main() {
    let mut params = Params::from_args();
    search(params.clone()).expect("");


    let mut reader = Editor::<()>::new();
    if let Err(_) = reader.load_history("finna_history.txt") {
    }

    loop {
        let readline = reader.readline("> ");

        match readline {
            Ok(line) => {
                reader.add_history_entry(&line);
                match line.as_ref() {
                    ":q" => { break }
                    ":n" => {
                        params.page = params.page+1;
                        search(params.clone()).expect("");
                    }
                    &_ => {
                        let mut args = vec![""];
                        let mut input_args = line.split(" ").collect();
                        args.append(&mut input_args);
                        params = Params::from_iter(args);
                        search(params.clone()).expect("");                
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                reader.save_history("finna_history.txt").unwrap();
                break
            }
            Err(ReadlineError::Eof) => {
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
}
