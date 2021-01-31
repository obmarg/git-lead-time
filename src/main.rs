use std::env;

mod pull_requests;

fn main() {
    let token =
        env::var("GITHUB_TOKEN").expect("Provide a github token via the GITHUB_TOKEN env var");

    for pr in pull_requests::for_repo("obmarg", "cynic", token) {
        println!("{:?}", pr);
    }
}
