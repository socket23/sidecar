use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};
use std::{borrow::Cow, sync::Arc};
use thiserror::Error;
use tree_sitter::Tree;

#[derive(Debug, Error)]
pub enum TreePrinterError {
    #[error("No language configuration found for file: {0}")]
    MissingConfig(String),
    #[error("Failed to parse tree for file: {0}")]
    ParseError(String),
    #[error("Invalid tree structure: {0}")]
    InvalidTree(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

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

    /// Gets tree-sitter configuration for file
    fn get_ts_config<'a>(
        &self,
        ts_parser: &'a TSLanguageParsing,
    ) -> Result<&'a TSLanguageConfig, TreePrinterError> {
        ts_parser
            .for_file_path(&self.filename)
            .ok_or(TreePrinterError::MissingConfig(self.filename.clone()))
    }

    // todo: get tree from parser
    fn get_tree(&self, ts_config: &TSLanguageConfig) -> Result<Tree, TreePrinterError> {
        ts_config
            .get_tree_sitter_tree(&self.code.as_bytes())
            .ok_or(TreePrinterError::ParseError(self.code.clone()))
    }

    // split code into lines
    fn split_code_into_lines(&self) -> Vec<&str> {
        self.code.split('\n').collect()
    }

    // get lines count
    fn get_lines_count(&self) -> usize {
        self.split_code_into_lines().len()
    }
    // initialise output lines HashMap

    // initialise scopes, headers, nodes

    // get root node

    // walk tree

    // add lines of interest (lois)

    // add context()

    // format
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::io::Write;
    use tempfile::Builder;

    #[test]
    fn test_tree_context_default() {
        // Act
        let default_context = TreeContext::default();

        // Assert
        assert_eq!(default_context.filename, "");
        assert_eq!(default_context.code, "");
        assert_eq!(default_context.line_number, false);
        assert_eq!(default_context.parent_context, true);
        assert_eq!(default_context.child_context, true);
        assert_eq!(default_context.last_line, true);
        assert_eq!(default_context.margin, 3);
        assert_eq!(default_context.mark_lois, true);
        assert_eq!(default_context.header_max, 10);
        assert_eq!(default_context.show_top_of_file_parent_scope, false);
        assert_eq!(default_context.loi_pad, 1);
    }

    #[test]
    fn test_get_ts_config_success() {
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        let context = TreeContext::new("test.ts".to_string(), "".to_string());
        let config = context.get_ts_config(&ts_parsing).unwrap();

        assert_eq!(config.file_extensions.contains(&"ts"), true);
        assert_eq!(config.file_extensions.contains(&"tsx"), true);
    }

    #[test]
    fn test_get_ts_config_failure() {
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        let context = TreeContext::new("nonexistent.xyz".to_string(), "".to_string());
        let result = context.get_ts_config(&ts_parsing);

        assert!(result.is_err());
        match result {
            Err(TreePrinterError::MissingConfig(filename)) => {
                assert_eq!(filename, "nonexistent.xyz");
            }
            _ => panic!("Expected MissingConfig error"),
        }
    }

    #[test]
    fn test_get_tree_typescript() {
        let mut file = Builder::new()
            .prefix("test")
            .suffix(".ts")
            .rand_bytes(0)
            .tempfile()
            .unwrap();

        let test_content = r#"// data-structures.ts
        export interface Vector2D {
          x: number;
          y: number;
        }
        
        export interface Size {
          width: number;
          height: number;
        }
        
        // shape.ts
        import { Vector2D, Size } from "./data-structures";
        
        export abstract class Shape {
          protected color: string;
        
          constructor(color: string) {
            this.color = color;
          }
        
          abstract getArea(): number;
          abstract getPerimeter(): number;
          abstract scale(factor: number): void;
        
          getColor(): string {
            return this.color;
          }
        
          setColor(color: string): void {
            this.color = color;
          }
        }
        
        // rectangle.ts
        import { Shape } from "./shape";
        import { Vector2D, Size } from "./data-structures";
        
        export class Rectangle extends Shape {
          private position: Vector2D;
          private size: Size;
        
          constructor(position: Vector2D, size: Size, color: string) {
            super(color);
            this.position = position;
            this.size = size;
          }
        
          getArea(): number {
            return this.size.width * this.size.height;
          }
        
          getPerimeter(): number {
            return 2 * (this.size.width + this.size.height);
          }
        
          scale(factor: number): void {
            this.size.width *= factor;
            this.size.height *= factor;
          }
        
          getPosition(): Vector2D {
            return this.position;
          }
        
          setPosition(position: Vector2D): void {
            this.position = position;
          }
        
          getSize(): Size {
            return this.size;
          }
        
          setSize(size: Size): void {
            this.size = size;
          }
        }
        
        // circle.ts
        import { Shape } from "./shape";
        import { Vector2D } from "./data-structures";
        
        export class Circle extends Shape {
          private center: Vector2D;
          private radius: number;
        
          constructor(center: Vector2D, radius: number, color: string) {
            super(color);
            this.center = center;
            this.radius = radius;
          }
        
          getArea(): number {
            return Math.PI * this.radius * this.radius;
          }
        
          getPerimeter(): number {
            return 2 * Math.PI * this.radius;
          }
        
          scale(factor: number): void {
            this.radius *= factor;
          }
        
          getCenter(): Vector2D {
            return this.center;
          }
        
          setCenter(center: Vector2D): void {
            this.center = center;
          }
        
          getRadius(): number {
            return this.radius;
          }
        
          setRadius(radius: number): void {
            this.radius = radius;
          }
        }
        
        // main.ts
        import { Rectangle, Circle } from "./shapes";
        import { Vector2D, Size } from "./data-structures";
        
        const rectangle = new Rectangle({ x: 0, y: 0 }, { width: 10, height: 5 }, "red");
        console.log("Rectangle Area:", rectangle.getArea());
        console.log("Rectangle Perimeter:", rectangle.getPerimeter());
        
        const circle = new Circle({ x: 5, y: 5 }, 3, "blue");
        console.log("Circle Area:", circle.getArea());
        console.log("Circle Perimeter:", circle.getPerimeter());"#;

        file.write_all(test_content.as_bytes()).unwrap();

        let path = file.path().to_str().unwrap().to_string();

        let mut code = String::new();
        file.read_to_string(&mut code).unwrap();

        let ts_parsing = Arc::new(TSLanguageParsing::init());

        let context = TreeContext::new(path, code);

        let config = context.get_ts_config(&ts_parsing).unwrap();

        let tree = context.get_tree(&config);

        assert!(tree.is_ok());
        let tree = tree.unwrap();

        assert_eq!(tree.root_node().kind(), "program");
    }

    #[test]
    fn test_split_code_into_lines() {
        let str = "line1\nline2\nline3";
        let context = TreeContext::new("test.ts".to_string(), str.to_string());
        let lines: Vec<&str> = context.split_code_into_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }
}
