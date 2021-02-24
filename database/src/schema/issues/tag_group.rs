use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use migrations::sqlx::{self, FromRow};

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TagGroup {
    pub id: i32,
    pub name: String,
    pub single_select: bool,
    pub color: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Default, Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Taxonomy {
    pub tags: HashMap<i32, TaxonomyTag>,
    /// a map containing lowercase tag names and their corresponding ids
    pub lower_map: HashMap<String, i32>,
    pub ungrouped_tags: HashSet<i32>,
    pub tag_groups: HashMap<i32, TaxonomyGroup>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaxonomyGroup {
    pub name: String,
    pub single_select: bool,
    pub color: Option<String>,
    pub tags: HashSet<i32>,
}

impl TaxonomyGroup {
    fn new(name: String, color: Option<String>, single_select: bool) -> Self {
        Self {
            name,
            single_select,
            color,
            tags: Default::default(),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaxonomyTag {
    pub name: String,
    pub color: Option<String>,
}

struct TagAndGroup {
    tag_id: i32,
    tag: String,
    tag_color: Option<String>,
    group_id: Option<i32>,
    group: Option<String>,
    single_select: Option<bool>,
    group_color: Option<String>,
}

impl Taxonomy {
    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        let mut taxonomy = Self::default();

        for TagAndGroup {
            tag_id,
            tag,
            tag_color,
            group_id,
            group,
            single_select,
            group_color,
        } in sqlx::query_as!(
            TagAndGroup,
            r#"
            SELECT
                tags.id as tag_id,
                tags.name as tag,
                tags.color as "tag_color?",
                tag_groups.id as "group_id?",
                tag_groups.name as "group?",
                tag_groups.single_select as "single_select?",
                tag_groups.color as "group_color?"
            FROM
                tags
            LEFT OUTER JOIN tag_groups ON tag_groups.id = tags.tag_group_id
        "#
        )
        .fetch_all(executor)
        .await?
        {
            taxonomy.lower_map.insert(tag.to_lowercase(), tag_id);

            if let Some(group_id) = group_id {
                let group = taxonomy.tag_groups.entry(group_id).or_insert_with(|| {
                    TaxonomyGroup::new(group.unwrap(), group_color, single_select.unwrap())
                });
                group.tags.insert(tag_id);
            } else {
                taxonomy.ungrouped_tags.insert(tag_id);
            }

            taxonomy.tags.insert(
                tag_id,
                TaxonomyTag {
                    name: tag,
                    color: tag_color,
                },
            );
        }

        Ok(taxonomy)
    }

    pub fn suggested_tags(&self) -> Vec<String> {
        self.tags.values().map(|t| t.name.clone()).collect()
    }
}
