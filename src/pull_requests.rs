pub fn for_repo(
    repo_owner: impl Into<String>,
    repo_name: impl Into<String>,
    token: impl Into<String>,
) -> impl Iterator<Item = queries::PullRequest> {
    PullRequestPages::new(repo_owner, repo_name, token).flatten()
}

struct PullRequestPages {
    next_arguments: Option<queries::PRsArguments>,
    token: String,
    client: reqwest::blocking::Client,
}

impl PullRequestPages {
    fn new(
        repo_owner: impl Into<String>,
        repo_name: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        PullRequestPages {
            next_arguments: Some(queries::PRsArguments {
                repo_name: repo_name.into(),
                repo_owner: repo_owner.into(),
                pr_cursor: None,
            }),
            token: token.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Iterator for PullRequestPages {
    type Item = Vec<queries::PullRequest>;

    fn next(&mut self) -> Option<Self::Item> {
        use cynic::{http::ReqwestBlockingExt, QueryBuilder};

        if let Some(next_args) = &self.next_arguments {
            let query = queries::PRs::build(next_args);

            let response = self
                .client
                .post("https://api.github.com/graphql")
                .bearer_auth(&self.token)
                .header("User-Agent", "obmarg/git-lead-time")
                .run_graphql(query)
                .unwrap();

            let connection = response.data?.repository?.pull_requests;
            if connection.page_info.has_next_page {
                self.next_arguments.as_mut().unwrap().pr_cursor = connection.page_info.end_cursor;
            } else {
                self.next_arguments = None;
            }

            return Some(connection.nodes);
        }

        None

        // TODO: Decode response etc.
    }
}
