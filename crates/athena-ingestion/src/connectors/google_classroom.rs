//! Google Classroom connector (07_INTEGRATIONS.md §1.9, OAuth
//! amendment). Read-only sync of courses, coursework (assignments + due
//! dates), and announcements for courses the authenticated user is
//! already enrolled in/teaches — never a domain-wide roster scan, never
//! a grade write, never a submission action. Scopes:
//! `.../auth/classroom.courses.readonly`,
//! `.../auth/classroom.coursework.me.readonly`,
//! `.../auth/classroom.announcements.readonly`.

use serde::Deserialize;

use crate::error::IngestionError;

pub const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const SCOPE: &str = "https://www.googleapis.com/auth/classroom.courses.readonly \
https://www.googleapis.com/auth/classroom.coursework.me.readonly \
https://www.googleapis.com/auth/classroom.announcements.readonly";

#[derive(Debug, Clone, PartialEq)]
pub struct ClassroomCourse {
    pub course_id: String,
    pub name: String,
    pub section: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassroomCoursework {
    pub course_id: String,
    pub coursework_id: String,
    pub title: String,
    pub due_at: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassroomAnnouncement {
    pub course_id: String,
    pub announcement_id: String,
    pub text: Option<String>,
    pub posted_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CoursesResponse {
    courses: Option<Vec<CourseDto>>,
}
#[derive(Debug, Deserialize)]
struct CourseDto {
    id: String,
    name: String,
    section: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CourseWorkResponse {
    #[serde(rename = "courseWork")]
    course_work: Option<Vec<CourseWorkDto>>,
}
#[derive(Debug, Deserialize)]
struct CourseWorkDto {
    id: String,
    title: String,
    #[serde(rename = "dueDate")]
    due_date: Option<DueDate>,
    #[serde(rename = "dueTime")]
    due_time: Option<DueTime>,
    state: Option<String>,
}
#[derive(Debug, Deserialize)]
struct DueDate {
    year: i64,
    month: i64,
    day: i64,
}
#[derive(Debug, Deserialize)]
struct DueTime {
    hours: Option<i64>,
    minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AnnouncementsResponse {
    announcements: Option<Vec<AnnouncementDto>>,
}
#[derive(Debug, Deserialize)]
struct AnnouncementDto {
    id: String,
    text: Option<String>,
    #[serde(rename = "creationTime")]
    creation_time: Option<String>,
}

fn build_client(access_token: &str) -> Result<reqwest::Client, IngestionError> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {access_token}"))
        .map_err(|e| IngestionError::Parse(format!("classroom auth header: {e}")))?;
    headers.insert(AUTHORIZATION, value);

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| IngestionError::Network(format!("classroom client build: {e}")))
}

/// Combines Classroom's separate `dueDate`/`dueTime` objects (time is
/// optional — an assignment can be date-only) into one ISO-8601 instant,
/// matching every other timestamp field in this codebase.
fn format_due(due_date: &Option<DueDate>, due_time: &Option<DueTime>) -> Option<String> {
    let d = due_date.as_ref()?;
    let hour = due_time.as_ref().and_then(|t| t.hours).unwrap_or(0);
    let minute = due_time.as_ref().and_then(|t| t.minutes).unwrap_or(0);
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
        d.year, d.month, d.day, hour, minute
    ))
}

/// Active courses the authenticated user is enrolled in/teaches —
/// `courseStates=ACTIVE` narrows this to what's actually current, the
/// same "never a full account scan" discipline §1.3 already establishes
/// for GitHub, applied to Classroom's own shape (the scope itself
/// already limits this to the user's own courses, not a domain-wide
/// roster).
pub async fn fetch_courses(access_token: &str) -> Result<Vec<ClassroomCourse>, IngestionError> {
    let client = build_client(access_token)?;
    let url = "https://classroom.googleapis.com/v1/courses?courseStates=ACTIVE&pageSize=50";
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("classroom courses: {e}")))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(IngestionError::AuthExpired("classroom access token rejected".into()));
    }
    if !resp.status().is_success() {
        return Err(IngestionError::Network(format!(
            "classroom courses returned {}",
            resp.status()
        )));
    }

    let parsed: CoursesResponse = resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("classroom courses payload: {e}")))?;
    Ok(parsed
        .courses
        .unwrap_or_default()
        .into_iter()
        .map(|c| ClassroomCourse {
            course_id: c.id,
            name: c.name,
            section: c.section,
        })
        .collect())
}

/// Assignments + due dates for one course (§1.9's "Assignments, Due
/// dates"). The caller (`athena-app`) iterates every course from
/// `fetch_courses` independently — one course's coursework failing does
/// not abort sibling courses, same per-item degrade-path precedent as
/// GitHub's per-repo sync (§1.3/§5).
pub async fn fetch_coursework(
    access_token: &str,
    course_id: &str,
) -> Result<Vec<ClassroomCoursework>, IngestionError> {
    let client = build_client(access_token)?;
    let url = format!("https://classroom.googleapis.com/v1/courses/{course_id}/courseWork?pageSize=50");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("classroom coursework: {e}")))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(IngestionError::AuthExpired("classroom access token rejected".into()));
    }
    if !resp.status().is_success() {
        return Err(IngestionError::Network(format!(
            "classroom coursework returned {}",
            resp.status()
        )));
    }

    let parsed: CourseWorkResponse = resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("classroom coursework payload: {e}")))?;
    Ok(parsed
        .course_work
        .unwrap_or_default()
        .into_iter()
        .map(|c| ClassroomCoursework {
            course_id: course_id.to_string(),
            coursework_id: c.id,
            title: c.title,
            due_at: format_due(&c.due_date, &c.due_time),
            state: c.state,
        })
        .collect())
}

/// Announcements for one course (§1.9's "Announcements").
pub async fn fetch_announcements(
    access_token: &str,
    course_id: &str,
) -> Result<Vec<ClassroomAnnouncement>, IngestionError> {
    let client = build_client(access_token)?;
    let url = format!("https://classroom.googleapis.com/v1/courses/{course_id}/announcements?pageSize=50");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("classroom announcements: {e}")))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(IngestionError::AuthExpired("classroom access token rejected".into()));
    }
    if !resp.status().is_success() {
        return Err(IngestionError::Network(format!(
            "classroom announcements returned {}",
            resp.status()
        )));
    }

    let parsed: AnnouncementsResponse = resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("classroom announcements payload: {e}")))?;
    Ok(parsed
        .announcements
        .unwrap_or_default()
        .into_iter()
        .map(|a| ClassroomAnnouncement {
            course_id: course_id.to_string(),
            announcement_id: a.id,
            text: a.text,
            posted_at: a.creation_time,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_due_combines_date_and_time() {
        let due_date = Some(DueDate { year: 2026, month: 9, day: 1 });
        let due_time = Some(DueTime { hours: Some(23), minutes: Some(59) });
        assert_eq!(format_due(&due_date, &due_time).as_deref(), Some("2026-09-01T23:59:00Z"));
    }

    #[test]
    fn format_due_defaults_missing_time_to_midnight() {
        let due_date = Some(DueDate { year: 2026, month: 12, day: 25 });
        assert_eq!(format_due(&due_date, &None).as_deref(), Some("2026-12-25T00:00:00Z"));
    }

    #[test]
    fn format_due_is_none_without_a_due_date() {
        assert_eq!(format_due(&None, &None), None);
    }
}
