fn main() {
    let branch = gitutils::current_branch();
    println!("Current branch output: \n{}", branch);
}
