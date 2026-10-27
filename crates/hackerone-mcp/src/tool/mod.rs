//! Shared tool vocabulary: the closed tool set, the enums both the JSON Schema
//! and the parameter validators read, the schema/params modules built on them,
//! one module per wired tool body, and the router that reaches it.
//!
//! Everything here is parity-frozen against `trailofbits/hackerone-mcp`
//! (ADR-0001): names, argument names, enum members, sort-field mapping and
//! query-parameter names come from upstream verbatim.

pub mod change_state;
pub mod create_comment;
pub mod create_report;
pub mod escalate_report;
pub mod get_program;
pub mod get_program_scope_exclusions;
pub mod get_program_structured_scopes;
pub mod get_program_weaknesses;
pub mod get_report;
pub mod get_report_custom_field_values;
pub mod list_my_reports;
pub mod list_scopes;
pub mod params;
pub mod query_reports;
pub mod remove_escalation;
pub mod schema;
pub mod scope;
pub mod search_hacktivity;
pub mod search_programs;
pub mod update_custom_field_values;
pub mod update_cves;
pub mod update_reference;
pub mod update_title;

use hackerone_api::{CredentialClass, Error};
use serde::de::Error as _;
use serde_json::{Map, Value};

use crate::Clients;

use scope::ProgramScope;

/// The MCP tools: the upstream thirteen first, then the hacker-side tools
/// appended in the order ADR-0008 introduced them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolName {
    GetReport,
    GetReportCustomFieldValues,
    CreateReport,
    QueryReports,
    ListScopes,
    ChangeState,
    CreateComment,
    UpdateTitle,
    UpdateReference,
    UpdateCves,
    EscalateReport,
    RemoveEscalation,
    UpdateCustomFieldValues,
    SearchPrograms,
    GetProgram,
    GetProgramStructuredScopes,
    GetProgramScopeExclusions,
    GetProgramWeaknesses,
    ListMyReports,
    SearchHacktivity,
}

impl ToolName {
    /// Every tool, in advertised order: upstream's thirteen, then the
    /// hacker-side reads.
    pub const ALL: [Self; 20] = [
        Self::GetReport,
        Self::GetReportCustomFieldValues,
        Self::CreateReport,
        Self::QueryReports,
        Self::ListScopes,
        Self::ChangeState,
        Self::CreateComment,
        Self::UpdateTitle,
        Self::UpdateReference,
        Self::UpdateCves,
        Self::EscalateReport,
        Self::RemoveEscalation,
        Self::UpdateCustomFieldValues,
        Self::SearchPrograms,
        Self::GetProgram,
        Self::GetProgramStructuredScopes,
        Self::GetProgramScopeExclusions,
        Self::GetProgramWeaknesses,
        Self::ListMyReports,
        Self::SearchHacktivity,
    ];

    /// Wire name as it appears in `tools/list` and `tools/call`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GetReport => "get_report",
            Self::GetReportCustomFieldValues => "get_report_custom_field_values",
            Self::CreateReport => "create_report",
            Self::QueryReports => "query_reports",
            Self::ListScopes => "list_scopes",
            Self::ChangeState => "change_state",
            Self::CreateComment => "create_comment",
            Self::UpdateTitle => "update_title",
            Self::UpdateReference => "update_reference",
            Self::UpdateCves => "update_cves",
            Self::EscalateReport => "escalate_report",
            Self::RemoveEscalation => "remove_escalation",
            Self::UpdateCustomFieldValues => "update_custom_field_values",
            Self::SearchPrograms => "search_programs",
            Self::GetProgram => "get_program",
            Self::GetProgramStructuredScopes => "get_program_structured_scopes",
            Self::GetProgramScopeExclusions => "get_program_scope_exclusions",
            Self::GetProgramWeaknesses => "get_program_weaknesses",
            Self::ListMyReports => "list_my_reports",
            Self::SearchHacktivity => "search_hacktivity",
        }
    }

    /// Look up a tool by its wire name.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// JSON Schema advertised for this tool's arguments.
    #[must_use]
    pub fn schema(self) -> Value {
        schema::schema(self)
    }

    /// Credential class this tool spends (ADR-0008).
    ///
    /// Exhaustive, so a new variant cannot compile without stating its class.
    #[must_use]
    pub const fn class(self) -> CredentialClass {
        match self {
            Self::GetReport
            | Self::GetReportCustomFieldValues
            | Self::CreateReport
            | Self::QueryReports
            | Self::ListScopes
            | Self::ChangeState
            | Self::CreateComment
            | Self::UpdateTitle
            | Self::UpdateReference
            | Self::UpdateCves
            | Self::EscalateReport
            | Self::RemoveEscalation
            | Self::UpdateCustomFieldValues => CredentialClass::Customer,
            Self::SearchPrograms
            | Self::GetProgram
            | Self::GetProgramStructuredScopes
            | Self::GetProgramScopeExclusions
            | Self::GetProgramWeaknesses
            | Self::ListMyReports
            | Self::SearchHacktivity => CredentialClass::Hacker,
        }
    }
}

/// Tools whose `run` exists, in advertised order, with their advertised
/// descriptions and credential class: the only tools `tools/list` may
/// advertise, and the authority `dispatch` uses to pick a client (ADR-0008).
/// Every name `ToolName` knows is wired.
pub const WIRED: [(ToolName, &str); 20] = [
    (
        ToolName::GetReport,
        "Fetch a single HackerOne report by ID.",
    ),
    (
        ToolName::GetReportCustomFieldValues,
        "Fetch normalized custom field values currently set on a single report.",
    ),
    (
        ToolName::CreateReport,
        "Import a known vulnerability report into the configured program.",
    ),
    (
        ToolName::QueryReports,
        "Query paginated HackerOne reports for the configured program using common \
         filters, sort, and pagination.",
    ),
    (
        ToolName::ListScopes,
        "List structured scopes for the configured program using common filters and \
         pagination.",
    ),
    (
        ToolName::ChangeState,
        "Change the state of a report (e.g. triaged, resolved, not-applicable, \
         duplicate, informative, spam).",
    ),
    (
        ToolName::CreateComment,
        "Add a comment to a report. Can be internal (team-only) or external (visible \
         to reporter).",
    ),
    (ToolName::UpdateTitle, "Update the title of a report."),
    (
        ToolName::UpdateReference,
        "Update the external reference (e.g. Jira ticket URL) on a report.",
    ),
    (
        ToolName::UpdateCves,
        "Update CVE identifiers associated with a report.",
    ),
    (
        ToolName::EscalateReport,
        "Escalate a report for higher-priority attention.",
    ),
    (
        ToolName::RemoveEscalation,
        "Remove an escalation from a report.",
    ),
    (
        ToolName::UpdateCustomFieldValues,
        "Set custom field values on a report.",
    ),
    (
        ToolName::SearchPrograms,
        "Search the HackerOne program directory. The API has no server-side filters or sort, so \
         offers_bounties, state and submission_state are matched client-side over a capped scan of \
         at most max_pages pages (default 5, cap 20). A result with truncated true means the scan \
         budget ran out while more directory pages existed; it does not mean more matches exist.",
    ),
    (
        ToolName::GetProgram,
        "Fetch one HackerOne program by handle, including offers_bounties, policy and \
         submission_state.",
    ),
    (
        ToolName::GetProgramStructuredScopes,
        "List a program's structured scopes with id and timestamp filters. The API returns at most \
         10 000 scopes per query; to read beyond that cap, pass scope_id_gt set to the last scope ID \
         seen and repeat.",
    ),
    (
        ToolName::GetProgramScopeExclusions,
        "List a program's scope exclusions: the report categories it never rewards, in \
         addition to core ineligible findings.",
    ),
    (
        ToolName::GetProgramWeaknesses,
        "List the weaknesses (CWEs) a program accepts, with page_number and page_size \
         (1-100, default 25).",
    ),
    (
        ToolName::ListMyReports,
        "List the authenticated hacker's own reports, with page_number and page_size \
         (1-100, default 25).",
    ),
    (
        ToolName::SearchHacktivity,
        "Search public HackerOne hacktivity with an Apache Lucene queryString (for example \
         'severity_rating:critical AND disclosed:true'), sorted by sort (default \
         -latest_disclosable_activity_at) and paginated.",
    ),
];

/// Route one `tools/call` to the tool that owns its body (ADR-0007), spending
/// the client for the tool's credential class (ADR-0008).
///
/// # Errors
///
/// Every failure a tool body produces, [`Error::MissingCredential`] when the
/// tool's class has no configured token pair, plus [`Error::Validation`] for an
/// unknown tool name.
pub async fn dispatch(
    clients: &Clients,
    scope: &ProgramScope,
    name: &str,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let Some(tool) = ToolName::from_wire(name) else {
        return Err(Error::Validation {
            field: "name".to_owned(),
            reason: format!("unknown tool: {name}"),
        });
    };
    let class = tool.class();
    let client = clients
        .for_class(class)
        .ok_or(Error::MissingCredential { class })?;
    match tool {
        ToolName::GetReport => get_report::run(client, args).await,
        ToolName::GetReportCustomFieldValues => {
            get_report_custom_field_values::run(client, args).await
        }
        ToolName::CreateReport => create_report::run(client, scope, args).await,
        ToolName::QueryReports => query_reports::run(client, scope, args).await,
        ToolName::ListScopes => list_scopes::run(client, scope, args).await,
        ToolName::ChangeState => change_state::run(client, args).await,
        ToolName::CreateComment => create_comment::run(client, args).await,
        ToolName::UpdateTitle => update_title::run(client, args).await,
        ToolName::UpdateReference => update_reference::run(client, args).await,
        ToolName::UpdateCves => update_cves::run(client, args).await,
        ToolName::EscalateReport => escalate_report::run(client, args).await,
        ToolName::RemoveEscalation => remove_escalation::run(client, args).await,
        ToolName::UpdateCustomFieldValues => update_custom_field_values::run(client, args).await,
        ToolName::SearchPrograms => search_programs::run(client, args).await,
        ToolName::GetProgram => get_program::run(client, args).await,
        ToolName::GetProgramStructuredScopes => {
            get_program_structured_scopes::run(client, args).await
        }
        ToolName::GetProgramScopeExclusions => {
            get_program_scope_exclusions::run(client, args).await
        }
        ToolName::GetProgramWeaknesses => get_program_weaknesses::run(client, args).await,
        ToolName::ListMyReports => list_my_reports::run(client, args).await,
        ToolName::SearchHacktivity => search_hacktivity::run(client, args).await,
    }
}

/// Required integer `report_id` argument, shared by the report-read tools.
pub(crate) fn report_id(args: &Map<String, Value>) -> Result<i64, Error> {
    let invalid = || Error::Validation {
        field: "report_id".to_owned(),
        reason: "must be an integer".to_owned(),
    };
    match args.get("report_id") {
        Some(Value::Number(number)) => number.as_i64().ok_or_else(invalid),
        Some(_) => Err(invalid()),
        None => Err(Error::Validation {
            field: "report_id".to_owned(),
            reason: "is required".to_owned(),
        }),
    }
}

/// A malformed API payload, naming the offending path (ADR-0003 wart fix: no
/// `{}`/`[]` coercion, so a broken body can never masquerade as an empty one).
pub(crate) fn decode_error(path: &str, reason: &str) -> Error {
    Error::Decode(serde_json::Error::custom(format!(
        "malformed payload at {path}: {reason}"
    )))
}

/// Map a 404 from a `{handle}` address to [`Error::ProgramNotFound`]; every
/// other failure passes through untouched.
pub(crate) fn not_found(handle: &str, error: Error) -> Error {
    match error {
        Error::Api { status: 404, .. } => Error::ProgramNotFound {
            handle: handle.to_owned(),
        },
        other => other,
    }
}

/// Canonical report states (upstream order); drives schema and validation.
pub const REPORT_STATES: [&str; 10] = [
    "new",
    "pending-program-review",
    "triaged",
    "needs-more-info",
    "resolved",
    "not-applicable",
    "informative",
    "duplicate",
    "spam",
    "retesting",
];

/// Severity ratings (upstream order); drives schema and validation.
pub const SEVERITIES: [&str; 5] = ["none", "low", "medium", "high", "critical"];

/// Permitted `sort_order` values.
pub const SORT_ORDERS: [&str; 2] = ["asc", "desc"];

/// One `query_reports` sort field: the tool argument and the `sort` query value.
#[derive(Debug, Clone, Copy)]
pub struct SortField {
    /// Argument name accepted from the caller.
    pub name: &'static str,
    /// Value sent as `sort` (table-qualified upstream field).
    pub query: &'static str,
}

/// Sort fields in upstream order.
pub const SORT_FIELDS: [SortField; 11] = [
    SortField {
        name: "swag_awarded_at",
        query: "reports.swag_awarded_at",
    },
    SortField {
        name: "bounty_awarded_at",
        query: "reports.bounty_awarded_at",
    },
    SortField {
        name: "last_reporter_activity_at",
        query: "reports.last_reporter_activity_at",
    },
    SortField {
        name: "first_program_activity_at",
        query: "reports.first_program_activity_at",
    },
    SortField {
        name: "last_program_activity_at",
        query: "reports.last_program_activity_at",
    },
    SortField {
        name: "triaged_at",
        query: "reports.triaged_at",
    },
    SortField {
        name: "created_at",
        query: "reports.created_at",
    },
    SortField {
        name: "closed_at",
        query: "reports.closed_at",
    },
    SortField {
        name: "last_public_activity_at",
        query: "reports.last_public_activity_at",
    },
    SortField {
        name: "last_activity_at",
        query: "reports.last_activity_at",
    },
    SortField {
        name: "disclosed_at",
        query: "reports.disclosed_at",
    },
];

/// Item validation applied to a `query_reports` array filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayFilterKind {
    /// Non-negative integers, serialized as strings.
    Integers,
    /// Free-form strings.
    Strings,
    /// Members of [`REPORT_STATES`].
    States,
    /// Members of [`SEVERITIES`].
    Severities,
}

/// One repeated query parameter: argument name, query name, item validation.
#[derive(Debug, Clone, Copy)]
pub struct ArrayFilter {
    /// Argument name accepted from the caller.
    pub argument: &'static str,
    /// Query key, repeated once per item.
    pub query: &'static str,
    /// Schema description for the argument.
    pub description: &'static str,
    /// Item validation.
    pub kind: ArrayFilterKind,
}

/// One timestamp query filter: argument name, query name, schema description.
#[derive(Debug, Clone, Copy)]
pub struct DateFilter {
    /// Argument name accepted from the caller.
    pub argument: &'static str,
    /// Query key.
    pub query: &'static str,
    /// Schema description for the argument.
    pub description: &'static str,
}

/// `query_reports` array filters, in upstream order.
pub const QUERY_REPORT_ARRAY_FILTERS: [ArrayFilter; 6] = [
    ArrayFilter {
        argument: "inbox_ids",
        query: "filter[inbox_ids][]",
        description: "Optional inbox IDs to narrow the query within the configured program.",
        kind: ArrayFilterKind::Integers,
    },
    ArrayFilter {
        argument: "reporters",
        query: "filter[reporter][]",
        description: "Reporter usernames to filter on.",
        kind: ArrayFilterKind::Strings,
    },
    ArrayFilter {
        argument: "assignees",
        query: "filter[assignee][]",
        description: "Assignee usernames, emails, or group names to filter on.",
        kind: ArrayFilterKind::Strings,
    },
    ArrayFilter {
        argument: "states",
        query: "filter[state][]",
        description: "Current report states to include.",
        kind: ArrayFilterKind::States,
    },
    ArrayFilter {
        argument: "report_ids",
        query: "filter[id][]",
        description: "Specific report IDs to fetch.",
        kind: ArrayFilterKind::Integers,
    },
    ArrayFilter {
        argument: "severities",
        query: "filter[severity][]",
        description: "Severity ratings to filter on.",
        kind: ArrayFilterKind::Severities,
    },
];

/// `query_reports` timestamp filters, in upstream order.
pub const QUERY_REPORT_DATE_FILTERS: [DateFilter; 10] = [
    DateFilter {
        argument: "created_after",
        query: "filter[created_at__gt]",
        description: "Only include reports created after this timestamp.",
    },
    DateFilter {
        argument: "created_before",
        query: "filter[created_at__lt]",
        description: "Only include reports created before this timestamp.",
    },
    DateFilter {
        argument: "submitted_after",
        query: "filter[submitted_at__gt]",
        description: "Only include reports submitted after this timestamp.",
    },
    DateFilter {
        argument: "submitted_before",
        query: "filter[submitted_at__lt]",
        description: "Only include reports submitted before this timestamp.",
    },
    DateFilter {
        argument: "triaged_after",
        query: "filter[triaged_at__gt]",
        description: "Only include reports triaged after this timestamp.",
    },
    DateFilter {
        argument: "triaged_before",
        query: "filter[triaged_at__lt]",
        description: "Only include reports triaged before this timestamp.",
    },
    DateFilter {
        argument: "closed_after",
        query: "filter[closed_at__gt]",
        description: "Only include reports closed after this timestamp.",
    },
    DateFilter {
        argument: "closed_before",
        query: "filter[closed_at__lt]",
        description: "Only include reports closed before this timestamp.",
    },
    DateFilter {
        argument: "last_activity_after",
        query: "filter[last_activity_at__gt]",
        description: "Only include reports with activity after this timestamp.",
    },
    DateFilter {
        argument: "last_activity_before",
        query: "filter[last_activity_at__lt]",
        description: "Only include reports with activity before this timestamp.",
    },
];

/// `list_scopes` timestamp filters, in upstream order.
pub const LIST_SCOPES_DATE_FILTERS: [DateFilter; 2] = [
    DateFilter {
        argument: "created_after",
        query: "filter[created_at__gt]",
        description: "Only include scopes created after this timestamp.",
    },
    DateFilter {
        argument: "updated_after",
        query: "filter[updated_at__gt]",
        description: "Only include scopes updated after this timestamp.",
    },
];
