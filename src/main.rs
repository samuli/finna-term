use std::collections::HashMap;

#[macro_use]
extern crate serde;
extern crate serde_derive;
extern crate reqwest;
extern crate colored;

use dialoguer::Input;
use colored::*;
use reqwest::Error;
use serde_json::Value;
use structopt::StructOpt;
//use itertools::Itertools;

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
    pub status: String,
    #[serde(default)]
    pub records: Vec<Record>,
    pub result_count: i32,
}

#[derive(StructOpt)]
struct Cli {
    lookfor: String,
}

struct Params {
    pub lookfor: String,
    pub limit: i32,
    pub page: i32,
    pub lng: String        
}

fn vec2str(vec: &Vec<String>, delimiter: &str) -> String {
    if vec.len() == 1 {
        vec[0].clone()
    } else {
        let tot = vec.len();
        vec.iter().enumerate().fold(String::new(), |acc, (i, arg)| {
            let mut res = acc + &arg;
            if i < tot-1 {
                res = res + delimiter;
            }
            return res;
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
    let building = &rec.buildings.first().expect("no building found").translated;
    
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
fn search(params: &Params) -> Result<(), Error> {
    let request_url = format!("https://api.finna.fi/api/v1/search?lookfor={lookfor}&type=AllFields&sort=relevance%2Cid%20asc&page={page}&limit={limit}&prettyPrint=false&lng=fi&field[]=id&field[]=title&field[]=formats&field[]=authors&field[]=buildings&field[]=nonPresenterAuthors&lng={lng}",
                              lookfor = params.lookfor,
                              page = params.page,
                              limit = params.limit,
                              lng = params.lng);
    println!("{}", request_url);
    let mut response = reqwest::get(&request_url).expect("Error reading from API");

    let results: SearchResults = response.json().expect("Error parsing results");
    render_results(&params, &results);
    Ok(())
}


#[derive(StructOpt)]
pub struct Cmd {
    lookfor: Option<String>,
    #[structopt(short = "f", long = "filter")]
    filter: Option<String>
}

fn main() {
    let args = Cli::from_args();

    let mut params = Params {
        lookfor: args.lookfor,
        limit: 20,
        page: 1,
        lng: "fi".to_string()
    };
    
    search(&params).expect("");


    loop {
        let input: String = Input::new().with_prompt("> ").interact().unwrap();
        match input.as_ref() {
            ":q" => { break }
            ":n" => {
                params.page = params.page+1;
                search(&params).expect("");                
            }
            &_ => {
                let cmd = Cmd::from_iter(input.split(" "));
                //println!("{}", cmd.lookfor.unwrap());
                match cmd.filter {
                    Some(filter) => {
                        println!("{}", filter);
                    }
                    _ => {}
                }

                params.lookfor = cmd.lookfor.unwrap();
                params.page = 1;
                search(&params).expect("");                
            }
        }
    }
    
}
