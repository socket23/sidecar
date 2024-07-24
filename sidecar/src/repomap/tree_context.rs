use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
};

use tree_sitter::{Node, Tree, TreeCursor};

use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};

use super::tree_walker::{self, TreeWalker, TreeWalker2};

pub struct TreeContext<'a> {
    // filename: String,
    code: String,
    parent_context: bool,
    child_context: bool,
    last_line: bool,
    margin: usize,
    mark_lois: bool,
    header_max: usize,
    show_top_of_file_parent_scope: bool,
    loi_pad: usize,
    output: Vec<String>,
    // config: TSLanguageConfig,
    lois: HashSet<usize>,
    show_lines: HashSet<usize>, // row numbers
    num_lines: usize,
    lines: Vec<String>,
    line_number: bool,
    done_parent_scopes: HashSet<usize>,
    nodes: Vec<Vec<Node<'a>>>,
    scopes: Vec<HashSet<usize>>, // the starting lines of the nodes that span the line
    header: Vec<Vec<(usize, usize, usize)>>, // the size, start line, end line of the nodes that span the line
    output_lines: HashMap<usize, String>,
}

impl<'a> TreeContext<'a> {
    pub fn new(code: String) -> Self {
        // let ts_parsing = TSLanguageParsing::init();
        // let config = ts_parsing.for_file_path(&filename).unwrap().clone();
        let lines: Vec<String> = code.split('\n').map(|s| s.to_string()).collect();
        let num_lines = lines.len() + 1;

        Self {
            code,
            parent_context: true,
            child_context: true,
            last_line: true,
            margin: 3,
            mark_lois: true,
            header_max: 10,
            show_top_of_file_parent_scope: false,
            loi_pad: 1,
            output: vec![],
            lois: HashSet::new(),
            show_lines: HashSet::new(),
            num_lines,
            lines,
            done_parent_scopes: HashSet::new(),
            scopes: vec![HashSet::new(); num_lines],
            header: vec![Vec::new(); num_lines],
            line_number: false,
            output_lines: HashMap::new(),
            nodes: vec![vec![]; num_lines],
        }
    }

    pub fn init(&mut self, cursor: TreeCursor<'a>) {
        self.walk(cursor);
        self.arrange_headers();
    }

    pub fn walk(&mut self, mut cursor: TreeCursor<'a>) {
        // It is dfs in a way
        loop {
            let start_line = cursor.node().start_position().row;
            let end_line = cursor.node().end_position().row;
            let size = end_line - start_line;

            self.nodes[start_line].push(cursor.node());

            if size > 0 {
                println!(
                    "tree_context::header_insertion::line_number({})::({}-{})",
                    start_line, start_line, end_line
                );
                self.header[start_line].push((size, start_line, end_line));
            }

            for i in start_line..=end_line {
                self.scopes[i].insert(start_line);
            }

            // Try to move to the first child
            if cursor.goto_first_child() {
                continue;
            }

            // If no children, try to move to the next sibling
            if cursor.goto_next_sibling() {
                continue;
            }

            // If no next sibling, go up the tree
            loop {
                if !cursor.goto_parent() {
                    // We've reached the root again, we're done
                    return;
                }

                // go to next sibling, break to continue outer loop
                if cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    // pub fn get_config(&self) -> &TSLanguageConfig {
    //     &self.config
    // }

    pub fn get_lois(&self) -> &HashSet<usize> {
        &self.lois
    }

    /// add lines of interest to the context
    pub fn add_lois(&mut self, lois: Vec<usize>) {
        self.lois.extend(lois);
    }

    pub fn print_state(&self) {
        // we want to see the headers which have been set
        // we want to see the lois
        // the nodes which have been set
        // scopes is the most important
        println!("tree_context::scope_debugging");
        self.scopes
            .iter()
            .enumerate()
            .for_each(|(line_number, values)| {
                if !values.is_empty() {
                    println!(
                        "scope::({})::({})",
                        line_number + 1,
                        values
                            .iter()
                            .map(|value| (value + 1).to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            });
    }

    pub fn add_context(&mut self) {
        if self.lois.is_empty() {
            return;
        }

        self.show_lines = self.lois.clone();

        if self.loi_pad > 0 {
            // for each interesting line
            for line in self.show_lines.clone().iter() {
                // for each of their surrounding lines
                for new_line in
                    line.saturating_sub(self.loi_pad)..=line.saturating_add(self.loi_pad)
                // since new_line usize could be negative
                {
                    if new_line >= self.num_lines {
                        continue;
                    }

                    self.show_lines.insert(new_line);
                }
            }
        }

        if self.last_line {
            // add the bottom line
            // we are adding \n and then we need to go back to 0 based indexing
            // so we need to do - 2
            let bottom_line = self.num_lines - 2;
            self.show_lines.insert(bottom_line);
            self.add_parent_scopes(bottom_line, vec![]);
        }

        if self.parent_context {
            for index in self.lois.clone().iter() {
                self.add_parent_scopes(*index, vec![]);
            }
        }

        if self.child_context {
            for index in self.lois.clone().iter() {
                self.add_child_context(*index);
            }
        }

        // this shows the top of the lines
        if self.margin > 0 {
            self.show_lines.extend(0..self.margin);
        }

        let mut line_numbers = self.show_lines.clone().into_iter().collect::<Vec<_>>();
        line_numbers.sort_unstable();
        println!(
            "tree_context::lois::({})",
            line_numbers
                .into_iter()
                .map(|line| (line + 1).to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        self.close_small_gaps();
    }

    // One of the bugs here is with this input:
    // tree_context::close_small_gaps::(0,1,2,6,7,8,9,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,37,45,46)
    // tree_context::close_small_gaps::closed::(0,1,2,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,37,38,45,46)
    fn close_small_gaps(&mut self) {
        // a "closing" operation on the integers in set.
        // if i and i+2 are in there but i+1 is not, I want to add i+1
        // Create a new set for the "closed" lines
        let mut closed_show = self.show_lines.clone();
        let mut sorted_show: Vec<usize> = self.show_lines.iter().cloned().collect();
        sorted_show.sort_unstable();
        println!(
            "tree_context::close_small_gaps::({})",
            sorted_show
                .iter()
                .map(|show| show.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        for (i, &value) in sorted_show.iter().enumerate().take(sorted_show.len() - 1) {
            if sorted_show[i + 1] - value == 2 {
                closed_show.insert(value + 1);
            }
        }

        // pick up adjacent blank lines
        for (i, line) in self.lines.iter().enumerate() {
            if !closed_show.contains(&i) {
                continue;
            }
            if !line.trim().is_empty()
                && i < self.num_lines - 2
                && self.lines[i + 1].trim().is_empty()
            {
                closed_show.insert(i + 1);
            }
        }

        let mut closed_closed_show = closed_show.clone().into_iter().collect::<Vec<_>>();
        closed_closed_show.sort_unstable();

        println!(
            "tree_context::close_small_gaps::closed::({})",
            closed_closed_show
                .iter()
                .map(|show| show.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        self.show_lines = closed_show;
    }

    fn add_child_context(&mut self, index: usize) {
        if self.nodes[index].is_empty() {
            return;
        }

        // are we getting the largest node here all the time??
        let last_line = self.get_last_line_of_scope(index);
        println!(
            "tree_context::add_child_context::line_index::({})::last_line::({})",
            index, last_line
        );
        let size = last_line - index;

        if size < 5 {
            self.show_lines.extend(index..=last_line); // inclusive
            return;
        }

        let mut children: Vec<Node> = vec![];

        // for all nodes that start at line[index], extend children.
        for node in self.nodes[index].iter() {
            children.extend(self.find_all_children(*node));
        }

        children.sort_by_key(|node| node.end_position().row - node.start_position().row);
        children.reverse();

        let currently_showing = self.show_lines.len();

        let max_to_show = 25;
        let min_to_show = 5;
        let percent_to_show = 0.10;
        let max_to_show = max(
            min((size as f64 * percent_to_show) as usize, max_to_show),
            min_to_show,
        );

        let child_start_lines: Vec<usize> = children
            .iter()
            .map(|child| child.start_position().row)
            .collect();

        // moved this because it was part of the for loop
        if self.show_lines.len() > currently_showing + max_to_show {
            return;
        }

        for &child_start_line in child_start_lines.iter() {
            self.add_parent_scopes(child_start_line, vec![]);
        }
    }

    fn find_all_children(&self, node: Node<'a>) -> Vec<Node<'a>> {
        let mut children = vec![node];
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            children.push(child);
        }

        children
    }

    fn get_last_line_of_scope(&self, index: usize) -> usize {
        self.nodes[index]
            .iter()
            .map(|node| node.end_position().row)
            .max()
            .unwrap()
    }

    pub fn format(&self) -> String {
        if self.show_lines.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        let mut dots = !(self.show_lines.contains(&0));

        for index in self.show_lines.iter() {
            if self.show_lines.contains(&index) {
                if dots {
                    if self.line_number {
                        output.push_str("...⋮...\n");
                    } else {
                        output.push_str("⋮...\n");
                    }

                    dots = false;
                }
            }

            if self.lois.contains(&index) && self.mark_lois {
                output.push_str("⋮...\n");
                continue;
            }

            let spacer = "|";

            let mut line_output = format!(
                "{}{}",
                spacer,
                self.output_lines.get(&index).unwrap_or(&self.lines[*index])
            );

            if self.line_number {
                line_output = format!("{:3}{}", index + 1, line_output);
            }

            output.push_str(&line_output);
            output.push('\n');

            dots = true;
        }

        output
    }

    // TODO: understand this completely since we are messing with states
    pub fn add_parent_scopes(&mut self, index: usize, recurse_depth: Vec<usize>) {
        if self.done_parent_scopes.contains(&index) {
            return;
        }

        self.done_parent_scopes.insert(index);

        // here for the scopes we are getting the headers and then figuring out
        // the first header (or the biggest one which we want to keep for the scope)
        println!(
            "tree_context::add_parent_scope::({})",
            recurse_depth
                .iter()
                .map(|depth| depth.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        for line_num in self.scopes[index].clone().iter() {
            // the header has 45-46 is very weird
            println!(
                "tree_context::add_parent_node::index::({})line_num::({})header::({})",
                index,
                line_num,
                self.header[*line_num]
                    .iter()
                    .map(|(size, head_start, head_end)| format!("[{head_start}-{head_end}]"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let (size, head_start, head_end) = self.header[*line_num].first().unwrap();

            if head_start > &0 || self.show_top_of_file_parent_scope {
                self.show_lines.extend(*head_start..*head_end);
            }

            if self.last_line {
                let mut recurse_depth_cloned = recurse_depth.clone();
                recurse_depth_cloned.push(index);
                let last_line = self.get_last_line_of_scope(*line_num);
                self.add_parent_scopes(last_line, recurse_depth_cloned);
            }
        }
    }

    fn arrange_headers(&mut self) {
        for i in 0..self.num_lines {
            println!(
                "tree_context::arrange_headers::header_len::({})::({})",
                i,
                self.header[i].len()
            );
            self.header[i].sort_unstable();

            // determine the header's start and end lines
            let start_end_maybe = if self.header[i].len() > 1 {
                let (size, start, end) = self.header[i][0];

                // if the node spans more than the max header size, curtail the header
                Some(if size > self.header_max {
                    (start, start + self.header_max)
                } else {
                    (start, end)
                })
            } else if self.header[i].len() == 0 {
                // if the node spans only one line
                Some((i, i + 1))
            } else {
                None
            };

            if let Some((start_line, end_line)) = start_end_maybe {
                // size is now redundant
                println!(
                    "tree_context::arrange_headers::line_index({})::({}-{})",
                    i, start_line, end_line
                );
                self.header[i] = vec![(0, start_line, end_line)];
            }
        }
    }
}
