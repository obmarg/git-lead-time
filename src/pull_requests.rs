pub fn pages_for_repo(
    repo_owner: impl Into<String>,
    repo_name: impl Into<String>,
    token: impl Into<String>,
) -> PullRequestPages {
    let client = reqwest::blocking::Client::new();

    let repo_owner = repo_owner.into();
    let repo_name = repo_name.into();
    let token = token.into();

    let total_count = get_total_count(&client, &token, &repo_name, &repo_owner);

    PullRequestPages::new(repo_owner, repo_name, token, total_count, client)
}

pub struct PullRequestPages {
    next_arguments: Option<queries::PRsArguments>,
    token: String,
    client: reqwest::blocking::Client,
    pub total_count: i32,
}

impl PullRequestPages {
    fn new(
        repo_owner: impl Into<String>,
        repo_name: impl Into<String>,
        token: impl Into<String>,
        total_count: i32,
        client: reqwest::blocking::Client,
    ) -> Self {
        PullRequestPages {
            next_arguments: Some(queries::PRsArguments {
                repo_name: repo_name.into(),
                repo_owner: repo_owner.into(),
                pr_cursor: None,
                page_size: 50,
            }),
            token: token.into(),
            total_count,
            client,
        }
    }
}

impl Iterator for PullRequestPages {
    type Item = Vec<queries::PullRequest>;

    fn next(&mut self) -> Option<Self::Item> {
        use cynic::QueryBuilder;

        if let Some(next_args) = &self.next_arguments {
            let query = queries::PRs::build(next_args);

            let response = run_query(&self.client, &self.token, query);

            let connection = response.data?.repository?.pull_requests;

            if connection.page_info.has_next_page {
                self.next_arguments.as_mut().unwrap().pr_cursor = connection.page_info.end_cursor;
            } else {
                self.next_arguments = None;
            }

            return Some(connection.nodes);
        }

        None
    }
}

fn get_total_count(
    client: &reqwest::blocking::Client,
    token: &str,
    repo_name: &str,
    repo_owner: &str,
) -> i32 {
    use cynic::QueryBuilder;

    let query = queries::PRs::build(queries::PRsArguments {
        repo_name: repo_name.to_string(),
        repo_owner: repo_owner.to_string(),
        pr_cursor: None,
        page_size: 1,
    });

    // Run a single query to get the # of PRs
    run_query(&client, &token, query)
        .data
        .unwrap()
        .repository
        .unwrap()
        .pull_requests
        .total_count
}

fn run_query(
    client: &reqwest::blocking::Client,
    token: &str,
    query: cynic::Operation<queries::PRs>,
) -> cynic::GraphQLResponse<queries::PRs> {
    use cynic::http::ReqwestBlockingExt;

    let response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(&token)
        .header("User-Agent", "obmarg/git-lead-time")
        .run_graphql(query)
        .unwrap();

    if let Some(errors) = &response.errors {
        if !errors.is_empty() {
            panic!("Errors: {:?}", errors);
        }
    }

    response
}
