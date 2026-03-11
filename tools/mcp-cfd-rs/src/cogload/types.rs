use serde::Serialize;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileScoreCategory {
    Negligible,
    Reviewable,
    Hotspot,
    HighHotspot,
    CriticalHotspot,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    Ignore,
    Review,
    ReduceWhenTouched,
    RefactorNow,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricComplexityCategory {
    Low,
    Moderate,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TotalComplexityCategory {
    Trivial,
    Moderate,
    High,
    Excessive,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileScore {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: f64,
    pub score_category: FileScoreCategory,
    pub recommended_action: RecommendedAction,
    /// `"ast"` when the `debtmap` crate parsed the file, `"heuristic"`
    /// otherwise.
    pub analysis_method: &'static str,
    /// Average cyclomatic complexity across functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_cyclomatic: Option<f64>,
    /// Maximum cyclomatic complexity among functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cyclomatic: Option<u32>,
    /// Average cognitive complexity across functions (AST only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_cognitive: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub todo_count: usize,
    pub max_indent_depth: usize,
    pub score: f64,
    pub score_category: FileScoreCategory,
    pub recommended_action: RecommendedAction,
    pub analysis_method: &'static str,
    pub top_todos: Vec<TodoEntry>,
    pub long_fn_lines: Vec<usize>,
    /// Per-function complexity breakdown (AST only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FunctionEntry>,
    /// Code smells detected by the `debtmap` crate (AST only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub code_smells: Vec<CodeSmellEntry>,
}

#[derive(Debug, Serialize)]
pub struct TodoEntry {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TouchedFilesReview {
    pub files: Vec<FileScore>,
    pub total_score: f64,
    pub skipped: Vec<SkippedFile>,
}

#[derive(Debug, Serialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeSmellEntry {
    pub line: usize,
    pub debt_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionEntry {
    pub name: String,
    pub line: usize,
    pub length: usize,
    pub cyclomatic: u32,
    pub cyclomatic_category: MetricComplexityCategory,
    pub cognitive: u32,
    pub cognitive_category: MetricComplexityCategory,
    pub nesting: u32,
    pub total_complexity: u32,
    pub total_complexity_category: TotalComplexityCategory,
    pub recommended_action: RecommendedAction,
}

#[derive(Debug, Serialize)]
pub struct FunctionComplexityReport {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub functions: Vec<FunctionEntry>,
    pub analysis_method: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CodeSmellReport {
    pub path: String,
    pub smells: Vec<CodeSmellEntry>,
    pub total: usize,
}
