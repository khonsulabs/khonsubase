use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use migrations::sqlx::{self, postgres::PgRow, FromRow, Row, Transaction};

use crate::schema::accounts::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueView {
    pub id: i64,
    pub author: User,
    pub summary: String,
    pub description: Option<String>,
    pub project_slug: Option<String>,
    pub project_name: Option<String>,
    pub parent_id: Option<i64>,
    pub current_revision_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl IssueView {
    pub async fn load(issue_id: i64) -> sqlx::Result<Self> {
        let row = sqlx::query!(
            r#"SELECT 
                issues.id, 
                accounts.id as author_id, 
                accounts.display_name as author_display_name, 
                accounts.username as author_username, 
                summary, 
                issues.description,
                projects.slug as "project_slug?",
                projects.name as "project_name?",
                parent_id, 
                current_revision_id, 
                issues.created_at, 
                completed_at 
               FROM issues
               INNER JOIN accounts ON issues.author_id = accounts.id 
               LEFT OUTER JOIN projects ON projects.id = project_id
               WHERE issues.id = $1"#,
            issue_id
        )
        .fetch_one(crate::pool())
        .await?;

        Ok(Self {
            id: row.id,
            author: User {
                id: row.author_id,
                username: row.author_username,
                display_name: row.author_display_name,
            },
            summary: row.summary,
            description: row.description,
            project_slug: row.project_slug,
            project_name: row.project_name,
            parent_id: row.parent_id,
            current_revision_id: row.current_revision_id,
            created_at: row.created_at,
            completed_at: row.completed_at,
        })
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Issue {
    pub id: i64,
    pub author_id: i64,
    pub summary: String,
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub current_revision_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Issue {
    pub fn new(
        author_id: i64,
        summary: String,
        description: Option<String>,
        parent_id: Option<i64>,
        project_id: Option<i64>,
    ) -> Self {
        Self {
            author_id,
            summary,
            description,
            parent_id,
            id: 0,
            project_id,
            current_revision_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub async fn load(issue_id: i64) -> sqlx::Result<Self> {
        sqlx::query_as!(Issue, "SELECT id, author_id, project_id, summary, description, parent_id, current_revision_id, created_at, completed_at FROM issues WHERE id = $1", issue_id).fetch_one(crate::pool()).await
    }

    pub async fn load_for_update(
        issue_id: i64,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(Issue, "SELECT id, author_id, project_id, summary, description, parent_id, current_revision_id, created_at, completed_at FROM issues WHERE id = $1 FOR UPDATE", issue_id).fetch_one(transaction).await
    }

    pub async fn save<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &mut self,
        executor: E,
    ) -> sqlx::Result<()> {
        if self.id == 0 {
            let row = sqlx::query!(
                r#"INSERT INTO issues (
                    author_id, 
                    project_id,
                    summary, 
                    description, 
                    parent_id,
                    current_revision_id,
                    completed_at
                   ) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, created_at"#,
                self.author_id,
                self.project_id,
                &self.summary,
                self.description.as_ref(),
                self.parent_id,
                self.current_revision_id,
                self.completed_at,
            )
            .fetch_one(executor)
            .await?;

            self.id = row.id;
            self.created_at = row.created_at;
        } else {
            sqlx::query!(
                r#"UPDATE issues SET 
                    author_id = $1,
                    summary = $2,
                    description = $3,
                    project_id = $4,
                    parent_id = $5,
                    current_revision_id = $6,
                    completed_at = $7
                   WHERE id = $8"#,
                self.author_id,
                &self.summary,
                self.description.as_ref(),
                self.project_id,
                self.parent_id,
                self.current_revision_id,
                self.completed_at,
                self.id,
            )
            .execute(executor)
            .await?;
        }

        Ok(())
    }

    pub async fn all_parents(issue_id: i64) -> sqlx::Result<Vec<Issue>> {
        let mut issues = HashMap::new();
        for issue in sqlx::query_as!(
            Issue,
            r#"WITH RECURSIVE issue_hierarchy AS(
                SELECT * FROM issues WHERE id = $1
                UNION ALL
                SELECT parent.* FROM issues parent JOIN issue_hierarchy ON parent.id = issue_hierarchy.parent_id
            )
            SELECT id as "id!", author_id as "author_id!", project_id, summary as "summary!", description, parent_id, current_revision_id, created_at as "created_at!", completed_at FROM issue_hierarchy"#,
            issue_id,
        ).fetch_all(crate::pool()).await? {
            issues.insert(issue.id, issue);
        }

        // Parents aren't guaranteed to be ordered in the order that their IDs are listed. Iterate up the chain to build the ordered list
        let mut ordered_issues = Vec::new();
        let mut id = issue_id;
        while let Some(issue) = issues.remove(&id) {
            let new_id = issue.parent_id;
            if id != issue_id {
                ordered_issues.insert(0, issue);
            }
            if let Some(new_id) = new_id {
                id = new_id
            } else {
                break;
            }
        }

        Ok(ordered_issues)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum IssueOrderingField {
    Id,
    Creation,
    Completion,
}

#[derive(Debug, Eq, PartialEq)]
pub struct IssueOrdering {
    pub field: IssueOrderingField,
    pub ascending: bool,
}

impl IssueOrdering {
    fn to_sql(&self) -> String {
        let order = if self.ascending { "ASC" } else { "DESC" };

        match self.field {
            IssueOrderingField::Id => format!("id {}", order),
            IssueOrderingField::Creation => format!("created_at {}", order),
            IssueOrderingField::Completion => format!("completed_at {0}, created_at {0}", order),
        }
    }
}

impl Default for IssueOrdering {
    fn default() -> Self {
        Self {
            field: IssueOrderingField::Creation,
            ascending: false,
        }
    }
}

#[derive(Default, Debug)]
pub struct IssueQueryBuilder {
    ordering: IssueOrdering,
    where_clauses: Vec<String>,
    pagination: IssuePagination,
}

#[derive(Debug)]
pub struct IssuePagination {
    pub page_size: usize,
    pub start_at: usize,
}

impl Default for IssuePagination {
    fn default() -> Self {
        Self {
            page_size: 50,
            start_at: 0,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IssueQueryResults {
    pub issues: Vec<Issue>,
    pub total_count: usize,
    pub start_at: usize,
    pub page_size: usize,
}

impl IssueQueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authored_by(mut self, author_id: i64) -> Self {
        self.where_clauses
            .push(format!("author_id = {}", author_id));
        self
    }

    pub fn completed(mut self) -> Self {
        self.where_clauses
            .push(String::from("completed_at IS NOT NULL"));
        self
    }

    pub fn open(mut self) -> Self {
        self.where_clauses
            .push(String::from("completed_at IS NULL"));
        self
    }

    pub fn order_by(mut self, ordering: IssueOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    pub fn project(mut self, project_id: Option<i64>) -> Self {
        if let Some(project_id) = project_id {
            self.where_clauses
                .push(format!("project_id = {}", project_id));
        } else {
            self.where_clauses.push(String::from("project_id IS NULL"));
        }
        self
    }

    pub fn owned_by(mut self, issue_id: Option<i64>) -> Self {
        if let Some(issue_id) = issue_id {
            self.where_clauses.push(format!("parent_id = {}", issue_id));
        } else {
            self.where_clauses.push(String::from("parent_id IS NULL"));
        }

        self
    }

    pub async fn query<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> sqlx::Result<IssueQueryResults> {
        let where_clauses = self.where_clauses.join(" AND ");
        let order_by = self.ordering.to_sql();
        let query = format!(
            r#"SELECT 
                id, 
                author_id, 
                summary, 
                description, 
                project_id,
                parent_id, 
                current_revision_id,
                created_at, 
                completed_at,
                count(*) OVER() as total_count
            FROM issues 
            WHERE {}
            ORDER BY {}
            LIMIT {}
            OFFSET {}"#,
            where_clauses, order_by, self.pagination.page_size, self.pagination.start_at
        );

        let rows: Vec<(i64, Issue)> = sqlx::query(&query)
            .map(|row: PgRow| {
                (
                    row.get("total_count"),
                    Issue {
                        id: row.get("id"),
                        author_id: row.get("author_id"),
                        summary: row.get("summary"),
                        description: row.get("description"),
                        project_id: row.get("project_id"),
                        parent_id: row.get("parent_id"),
                        current_revision_id: row.get("current_revision_id"),
                        created_at: row.get("created_at"),
                        completed_at: row.get("completed_at"),
                    },
                )
            })
            .fetch_all(executor)
            .await?;

        Ok(if rows.is_empty() {
            IssueQueryResults::default()
        } else {
            let total_count = rows[0].0 as usize;
            IssueQueryResults {
                total_count,
                start_at: self.pagination.start_at,
                page_size: self.pagination.page_size,
                issues: rows.into_iter().map(|(_, issue)| issue).collect(),
            }
        })
    }
}
