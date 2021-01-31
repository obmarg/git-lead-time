use std::env;

use anyhow::anyhow;
use structopt::StructOpt;

mod pull_requests;

#[derive(StructOpt)]
struct Opts {
    /// The GitHub organisation everything lives under
    org: String,

    /// The team name to query for
    team: String,

    /// The repository to query
    repo: String,
}

fn main() {
    let token =
        env::var("GITHUB_TOKEN").expect("Provide a github token via the GITHUB_TOKEN env var");

    let opts = Opts::from_args();

    let team_members = team_members(&opts.org, &opts.team, &token).unwrap();

    let pull_requests = pull_requests::for_repo(&opts.org, &opts.repo, token).filter(|pr| {
        if let Some(login) = pr.author.as_ref().and_then(|a| a.login()) {
            team_members.iter().any(|member| member == login)
        } else {
            false
        }
    });

    let lead_times = pull_requests
        .into_iter()
        .flat_map(|pr| {
            let check_suites = pr.merge_commit?.check_suites?.nodes;
            if check_suites
                .iter()
                .any(|suite| suite.status != queries::CheckStatusState::Completed)
            {
                // Tests either failed or not yet passed, skip this PR
                return None;
            }

            let deploy_time = check_suites
                .into_iter()
                .max_by_key(|suite| suite.updated_at.0)?
                .updated_at
                .0;

            Some(
                pr.commits
                    .nodes
                    .into_iter()
                    .map(move |commit| (commit.commit.authored_date.0, deploy_time)),
            )
        })
        .flatten()
        .map(|(commit_time, deploy_time)| (deploy_time - commit_time).num_minutes())
        .collect::<Vec<_>>();

    let num = lead_times.len() as i64;
    let total = lead_times.into_iter().sum::<i64>();

    println!("Mean Lead Time: {} minutes", total / num);
}

fn team_members(
    org: impl Into<String>,
    team: impl Into<String>,
    token: impl AsRef<str>,
) -> anyhow::Result<Vec<String>> {
    use cynic::{http::ReqwestBlockingExt, QueryBuilder};

    let query = queries::TeamMembers::build(queries::TeamMembersArguments {
        org: org.into(),
        team: team.into(),
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token.as_ref())
        .header("User-Agent", "obmarg/git-lead-time")
        .run_graphql(query)
        .unwrap();

    Ok(response
        .data
        .and_then(|r| r.organization)
        .and_then(|org| org.team)
        .map(|team| {
            team.members
                .nodes
                .into_iter()
                .map(|user| user.login)
                .collect()
        })
        .ok_or_else(|| anyhow!("couldn't find team members"))?)
}
