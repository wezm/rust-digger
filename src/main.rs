use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;

use chrono::prelude::*;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

mod read;
use read::{read_crate_owners, read_crates, read_teams, read_users};
mod render;
use render::{
    generate_crate_pages, generate_user_pages, load_templates, read_file,
    render_list_crates_by_repo, render_list_of_repos, render_list_page, render_news_pages,
    render_static_pages,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Repo {
    display: String,
    name: String,
    url: String,
    count: usize,
    percentage: String,
    crates: Vec<Crate>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Crate {
    created_at: String,
    description: String,
    documentation: String,
    downloads: String,
    homepage: String,
    id: String,
    max_upload_size: String,
    name: String,
    readme: String,
    repository: String,
    updated_at: String,

    #[serde(default = "empty_string")]
    owner_gh_login: String,

    #[serde(default = "empty_string")]
    owner_name: String,

    #[serde(default = "empty_string")]
    owner_gh_avatar: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    gh_avatar: String,
    gh_id: String,
    gh_login: String,
    id: String,
    name: String,

    #[serde(default = "get_zero")]
    count: u16,
}

fn empty_string() -> String {
    "".to_string()
}

fn get_zero() -> u16 {
    0
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Team {
    avatar: String,
    github_id: String,
    login: String,
    id: String,
    name: String,
    org_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CrateOwner {
    crate_id: String,
    created_at: String,
    created_by: String,
    owner_id: String,
    owner_kind: String,
}

//type RepoPercentage<'a> = HashMap<&'a str, String>;
type Owners = HashMap<String, String>;
type CratesByOwner = HashMap<String, Vec<String>>;
// type Users = HashMap<String, User>;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Starting the Rust Digger");
    log::info!("{VERSION}");

    let args: Vec<String> = env::args().collect();
    let limit;
    if args.len() == 2 {
        limit = args[1].parse().expect("Could not convert to i32");
    } else {
        limit = 0;
    }
    log::info!("Limit {limit}");

    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) = read_crate_owners(limit);
    let mut users = read_users(limit);
    read_teams(&mut users, limit);
    let mut crates: Vec<Crate> = read_crates(limit);
    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);

    generate_pages(&crates)?;
    render_news_pages();
    render_static_pages()?;
    generate_crate_pages(&crates)?;
    generate_user_pages(&crates, users, &crates_by_owner)?;

    log::info!("Ending the Rust Digger");
    Ok(())
}

fn add_owners_to_crates(crates: &mut Vec<Crate>, users: &Vec<User>, owner_by_crate_id: &Owners) {
    let mut mapping: HashMap<String, &User> = HashMap::new();
    for user in users {
        mapping.insert(user.id.clone(), user);
    }

    for krate in crates.into_iter() {
        let crate_id = &krate.id;
        match owner_by_crate_id.get(crate_id) {
            Some(owner_id) => {
                //println!("owner_id: {owner_id}");
                match mapping.get(owner_id) {
                    Some(val) => {
                        krate.owner_gh_login = val.gh_login.clone();
                        krate.owner_name = val.name.clone();
                        krate.owner_gh_avatar = val.gh_avatar.clone();
                    }
                    None => {
                        log::warn!("crate {crate_id} owner_id {owner_id} does not have a user");
                    }
                }
            }
            None => {
                log::warn!("crate {crate_id} does not have an owner");
            }
        }
    }
}

// fn has_repo(w: &Crate) -> bool {
//     w.repository != ""
// }
fn has_homepage_no_repo(w: &Crate) -> bool {
    w.homepage != "" && w.repository == ""
}
fn no_homepage_no_repo(w: &Crate) -> bool {
    w.homepage == "" && w.repository == ""
}

fn get_repo_types() -> Vec<Repo> {
    let repos: Vec<Repo> = vec![
        Repo {
            display: "GitHub".to_string(),
            name: "github".to_string(),
            url: "https://github.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "GitLab".to_string(),
            name: "gitlab".to_string(),
            url: "https://gitlab.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Codeberg".to_string(),
            name: "codeberg".to_string(),
            url: "https://codeberg.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Gitee".to_string(),
            name: "gitee".to_string(),
            url: "https://gitee.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Tor Project (GitLab)".to_string(),
            name: "torproject".to_string(),
            url: "https://gitlab.torproject.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Free Desktop (GitLab)".to_string(),
            name: "freedesktop".to_string(),
            url: "https://gitlab.freedesktop.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Wikimedia (GitLab)".to_string(),
            name: "wikimedia".to_string(),
            url: "https://gitlab.wikimedia.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "e3t".to_string(),
            name: "e3t".to_string(),
            url: "https://git.e3t.cc/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "srht".to_string(),
            name: "srht".to_string(),
            url: "https://git.sr.ht/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Open Privacy".to_string(),
            name: "openprivacy".to_string(),
            url: "https://git.openprivacy.ca/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Cronce (GitLab)".to_string(),
            name: "cronce".to_string(),
            url: "https://gitlab.cronce.io/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Gnome (GitLab)".to_string(),
            name: "gnome".to_string(),
            url: "https://gitlab.gnome.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Repo with http".to_string(),
            name: "repo-with-http".to_string(),
            url: "http://".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
    ];
    repos
}

fn collect_repos(crates: &Vec<Crate>) -> Vec<Repo> {
    let mut repos: Vec<Repo> = get_repo_types();
    let mut no_repo: Vec<Crate> = vec![];
    let mut other_repo: Vec<Crate> = vec![];

    for krate in crates {
        if krate.repository == "" {
            no_repo.push(krate.clone());
            continue;
        }
        let mut matched = false;
        repos = repos
            .into_iter()
            .map(|mut repo| {
                if krate.repository.starts_with(&repo.url) {
                    repo.count += 1;
                    matched = true;
                    repo.crates.push(krate.clone());
                }
                repo
            })
            .collect();

        if !matched {
            other_repo.push(krate.clone());
        }
    }

    repos.push(Repo {
        display: "Has no repository".to_string(),
        name: "no-repo".to_string(),
        url: "".to_string(),
        count: no_repo.len(),
        percentage: "0".to_string(),
        crates: no_repo,
    });

    repos.push(Repo {
        display: "Other repositories we don't recognize".to_string(),
        name: "other-repos".to_string(),
        url: "".to_string(),
        count: other_repo.len(),
        percentage: "0".to_string(),
        crates: other_repo,
    });

    repos = repos
        .into_iter()
        .map(|mut repo| {
            repo.percentage = percentage(repo.count, crates.len());
            repo
        })
        .collect();

    repos
}

fn percentage(num: usize, total: usize) -> String {
    let t = (10000 * num / total) as f32;
    (t / 100.0).to_string()
}

fn generate_pages(crates: &Vec<Crate>) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    // Create a folder _site
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");
    let _res = fs::create_dir_all("_site/news");
    let _res = fs::create_dir_all("_site/vcs");

    fs::copy("digger.js", "_site/digger.js")?;

    let all_crates: Vec<Crate> = crates.into_iter().cloned().collect();
    let home_page_but_no_repo = crates
        .into_iter()
        .filter(|w| has_homepage_no_repo(w))
        .cloned()
        .collect::<Vec<Crate>>();
    let no_homepage_no_repo_crates = crates
        .into_iter()
        .filter(|w| no_homepage_no_repo(w))
        .cloned()
        .collect::<Vec<Crate>>();
    let github_with_www = crates
        .into_iter()
        .filter(|w| w.repository != "" && w.repository.contains("www.github.com"))
        .cloned()
        .collect::<Vec<Crate>>();

    let repos = collect_repos(&crates);

    let mut partials = Partials::empty();
    let filename = "templates/incl/header.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/footer.html";
    partials.add(filename, read_file(filename));

    render_list_crates_by_repo(&repos)?;
    render_list_of_repos(&repos);

    render_list_page(
        &"_site/index.html".to_string(),
        &"Rust Digger".to_string(),
        &all_crates,
    )?;

    render_list_page(
        &"_site/github-with-www.html".to_string(),
        &"Github with www".to_string(),
        &github_with_www,
    )?;

    render_list_page(
        &"_site/has-homepage-but-no-repo.html".to_string(),
        &"Has homepage, but no repository".to_string(),
        &home_page_but_no_repo,
    )?;

    render_list_page(
        &"_site/no-homepage-no-repo.html".to_string(),
        &"No repository, no homepage".to_string(),
        &no_homepage_no_repo_crates,
    )?;

    //log::info!("repos: {:?}", repos);

    log::info!("render_stats_page");
    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/stats.html")
        .unwrap();

    let filename = "_site/stats.html";
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        //"user":    user,
        //"crate":   krate,
        "total": crates.len(),
        "repos": repos,
        "home_page_but_no_repo": home_page_but_no_repo.len(),
        "home_page_but_no_repo_percentage":  percentage(home_page_but_no_repo.len(), crates.len()),
        "no_homepage_no_repo_crates": no_homepage_no_repo_crates.len(),
        "no_homepage_no_repo_crates_percentage": percentage(no_homepage_no_repo_crates.len(), crates.len()),
        "github_with_www": github_with_www.len(),
        "github_with_www_percentage": percentage(github_with_www.len(), crates.len()),
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", html).unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_has_repo() {
    //     let x = Crate {
    //         created_at: "".to_string(),
    //         description: "".to_string(),
    //         documentation: "".to_string(),
    //         downloads: "".to_string(),
    //         homepage: "".to_string(),
    //         id: "".to_string(),
    //         max_upload_size: "".to_string(),
    //         name: "".to_string(),
    //         readme: "".to_string(),
    //         repository: "https://github.com/szabgab/rust-digger".to_string(),
    //         updated_at: "".to_string(),
    //     };
    //     assert!(has_repo(&x));

    //     let x = Crate {
    //         created_at: "".to_string(),
    //         description: "".to_string(),
    //         documentation: "".to_string(),
    //         downloads: "".to_string(),
    //         homepage: "".to_string(),
    //         id: "".to_string(),
    //         max_upload_size: "".to_string(),
    //         name: "".to_string(),
    //         readme: "".to_string(),
    //         repository: "".to_string(),
    //         updated_at: "".to_string(),
    //     };
    //     assert!(!has_repo(&x));
    // }

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(20, 100), "20");
        assert_eq!(percentage(5, 20), "25");
        assert_eq!(percentage(1234, 10000), "12.34");
        assert_eq!(percentage(1234567, 10000000), "12.34");
    }
}
