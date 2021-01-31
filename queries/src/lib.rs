pub use queries::*;

#[cynic::query_module(
    schema_path = r#"src/github.schema.graphql"#,
    query_module = "query_dsl"
)]
mod queries {
    use super::{query_dsl, types::*};

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct PRsArguments {
        pub repo_name: String,
        pub repo_owner: String,
        pub pr_cursor: Option<String>,
    }

    /// ```graphql
    /// query PRs($repoName: String!, $repoOwner: String!, $prCursor: String) {
    ///   repository(name: $repoName, owner: $repoOwner) {
    ///     pullRequests(first: 10, states: MERGED, after: $prCursor) {
    ///       pageInfo {
    ///         endCursor
    ///         hasNextPage
    ///       }
    ///       nodes {
    ///         commits(first: 250) {
    ///           nodes {
    ///             commit {
    ///               messageHeadline
    ///               authoredDate
    ///             }
    ///           }
    ///         }
    ///         mergeCommit {
    ///           messageHeadline
    ///           authoredDate
    ///           checkSuites(first: 25) {
    ///             nodes {
    ///               status
    ///               conclusion
    ///               updatedAt
    ///             }
    ///           }
    ///         }
    ///       }
    ///     }
    ///   }
    /// }
    /// ```
    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", argument_struct = "PRsArguments")]
    pub struct PRs {
        #[arguments(name = args.repo_name.clone(), owner = args.repo_owner.clone())]
        pub repository: Option<Repository>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Repository", argument_struct = "PRsArguments")]
    pub struct Repository {
        #[arguments(first = 10, states = Some(vec![PullRequestState::Merged]), after = &args.pr_cursor)]
        pub pull_requests: PullRequestConnection,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "PullRequestConnection")]
    pub struct PullRequestConnection {
        pub page_info: PageInfo,
        #[cynic(flatten)]
        pub nodes: Vec<PullRequest>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "PullRequest")]
    pub struct PullRequest {
        #[arguments(first = 250)]
        pub commits: PullRequestCommitConnection,
        pub merge_commit: Option<Commit>,
        pub author: Option<Actor>,
    }

    #[derive(cynic::InlineFragments, Debug)]
    #[cynic(graphql_type = "Actor")]
    pub enum Actor {
        User(User),

        #[cynic(fallback)]
        Other,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "User")]
    pub struct User {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "PullRequestCommitConnection")]
    pub struct PullRequestCommitConnection {
        #[cynic(flatten)]
        pub nodes: Vec<PullRequestCommit>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "PullRequestCommit")]
    pub struct PullRequestCommit {
        pub commit: Commit2,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "PageInfo")]
    pub struct PageInfo {
        pub end_cursor: Option<String>,
        pub has_next_page: bool,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Commit")]
    pub struct Commit {
        pub message_headline: String,
        pub authored_date: DateTime,
        #[arguments(first = 25)]
        pub check_suites: Option<CheckSuiteConnection>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Commit")]
    pub struct Commit2 {
        pub message_headline: String,
        pub authored_date: DateTime,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "CheckSuiteConnection")]
    pub struct CheckSuiteConnection {
        pub nodes: Option<Vec<Option<CheckSuite>>>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "CheckSuite")]
    pub struct CheckSuite {
        pub status: CheckStatusState,
        pub conclusion: Option<CheckConclusionState>,
        pub updated_at: DateTime,
    }

    #[derive(cynic::Enum, Clone, Copy, Debug)]
    #[cynic(graphql_type = "CheckConclusionState")]
    pub enum CheckConclusionState {
        ActionRequired,
        Cancelled,
        Failure,
        Neutral,
        Skipped,
        Stale,
        Success,
        TimedOut,
    }

    #[derive(cynic::Enum, Clone, Copy, Debug)]
    #[cynic(graphql_type = "CheckStatusState")]
    pub enum CheckStatusState {
        Completed,
        InProgress,
        Queued,
        Requested,
    }

    #[derive(cynic::Enum, Clone, Copy, Debug)]
    #[cynic(graphql_type = "PullRequestState")]
    pub enum PullRequestState {
        Closed,
        Merged,
        Open,
    }
}

mod types {
    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct Date(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct DateTime(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct GitObjectID(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct GitRefname(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct GitSSHRemote(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct GitTimestamp(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct Html(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct PreciseDateTime(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct Uri(pub String);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct X509Certificate(pub String);
}

mod query_dsl {
    use super::types::*;
    cynic::query_dsl!(r#"src/github.schema.graphql"#);
}
