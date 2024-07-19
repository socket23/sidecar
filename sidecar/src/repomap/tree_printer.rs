#[derive(Debug)]
struct TreeContext {
    filename: String,
    code: String,
    line_number: bool,
    parent_context: bool,
    child_context: bool,
    last_line: bool,
    margin: usize,
    mark_lois: bool,
    header_max: usize,
    show_top_of_file_parent_scope: bool,
    loi_pad: usize,
}

impl Default for TreeContext {
    fn default() -> Self {
        Self {
            filename: "".to_string(),
            code: "".to_string(),
            line_number: false,
            parent_context: true,
            child_context: true,
            last_line: true,
            margin: 3,
            mark_lois: true,
            header_max: 10,
            show_top_of_file_parent_scope: false,
            loi_pad: 1,
        }
    }
}

impl TreeContext {
    pub fn new(filename: String, code: String) -> Self {
        Self {
            filename,
            code,
            ..Default::default()
        }
    }

    // todo: get parser for language

    // todo: get tree from parser

    // split code into lines

    // get lines count

    // initialise output lines HashMap

    // initialise scopes, headers, nodes

    // get root node

    // walk tree

    // add lines of interest (lois)

    // add context()

    // format
}
