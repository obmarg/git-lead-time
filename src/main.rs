use std::env;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use console::style;
use indicatif::{HumanDuration, ProgressBar, ProgressIterator, ProgressStyle};
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

    let team_fetch = ProgressBar::new_spinner();
    team_fetch.set_message(&format!(
        "{} Fetching team members",
        style("[1/3]").bold().dim()
    ));

    let team_members = team_members(&opts.org, &opts.team, &token).unwrap();

    team_fetch.finish_with_message(&format!(
        "{} Fetched {} team members",
        style("[1/3]").bold().dim(),
        team_members.len()
    ));

    let pr_count = ProgressBar::new_spinner();
    pr_count.set_message(&format!(
        "{} Counting Pull Requests",
        style("[2/3]").bold().dim()
    ));
    let pr_pages = pull_requests::pages_for_repo(&opts.org, &opts.repo, token);
    pr_count.finish_with_message(&format!(
        "{} Counted {} Pull Requests",
        style("[2/3]").bold().dim(),
        pr_pages.total_count
    ));

    let pr_progress = ProgressBar::new(pr_pages.total_count as u64);
    pr_progress.set_style(
        ProgressStyle::default_bar().template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        ),
    );
    pr_progress.println(&format!(
        "  {} Fetching Pull Requests",
        style("[3/3]").bold().dim()
    ));

    let pull_requests = pr_pages
        .flatten()
        .progress_with(pr_progress.clone())
        .filter(|pr| {
            if let Some(login) = pr.author.as_ref().and_then(|a| a.login()) {
                team_members.iter().any(|member| member == login)
            } else {
                false
            }
        });

    let lead_times = {
        let mut lead_times = pull_requests
            .flat_map(|pr| {
                let deploy_time = pr.deploy_time()?;

                Some(
                    pr.commits
                        .nodes
                        .into_iter()
                        .map(move |commit| (commit.commit.authored_date, deploy_time)),
                )
            })
            .flatten()
            .map(|(commit_time, deploy_time)| (deploy_time - commit_time).num_seconds() as u64)
            .collect::<Vec<_>>();

        lead_times.sort_unstable();
        lead_times
    };

    pr_progress.finish_with_message(&format!("Fetched {} commits", lead_times.len()));

    if lead_times.is_empty() {
        println!("No commits found.  Giving up.");
        return;
    }

    let median_secs = lead_times[lead_times.len() / 2];
    let max_secs = *lead_times.last().unwrap();

    let num = lead_times.len() as u64;
    let total = lead_times.into_iter().sum::<u64>();
    let mean_seconds = total / num;

    println!("Total Commits: {}", num);
    println!(
        "Average Lead Time: {} ({} secs)",
        HumanDuration(std::time::Duration::from_secs(mean_seconds)),
        mean_seconds,
    );
    println!(
        "Median Lead Time: {} ({} secs)",
        HumanDuration(std::time::Duration::from_secs(median_secs)),
        median_secs,
    );
    println!(
        "Max Lead Time: {} ({} secs)",
        HumanDuration(std::time::Duration::from_secs(max_secs)),
        max_secs,
    );
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

#[easy_ext::ext]
impl queries::PullRequest {
    fn deploy_time(&self) -> Option<DateTime<Utc>> {
        self.status_finish_time()
            .or_else(|| self.check_suite_finish_time())
    }

    fn check_suite_finish_time(&self) -> Option<DateTime<Utc>> {
        let check_suites = &self.merge_commit.as_ref()?.check_suites.as_ref()?.nodes;
        if check_suites
            .iter()
            .any(|suite| suite.status != queries::CheckStatusState::Completed)
        {
            // Tests either failed or not yet passed, skip this PR
            return None;
        }

        Some(
            check_suites
                .iter()
                .max_by_key(|suite| suite.updated_at)?
                .updated_at,
        )
    }

    fn status_finish_time(&self) -> Option<DateTime<Utc>> {
        let status = self.merge_commit.as_ref()?.status.as_ref()?;
        if status.state != queries::StatusState::Success {
            // Tests either failed or not yet passed, skip this PR
            return None;
        }

        Some(
            status
                .contexts
                .iter()
                .max_by_key(|context| context.created_at)?
                .created_at,
        )
    }
}
