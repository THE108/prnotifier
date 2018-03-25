#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub email_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Reviewer {
    pub user: User,
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: u32,
    pub title: String,
    pub open: bool,
    pub created_date: i64,
    pub updated_date: i64,
    pub reviewers: Vec<Reviewer>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub size: u8,
    pub values: Vec<PullRequest>,
}
