// DO NOT EDIT: This file is autogenerated via the builtin command.

package array

import (
	ast "github.com/influxdata/flux/ast"
	runtime "github.com/influxdata/flux/runtime"
)

func init() {
	runtime.RegisterPackage(pkgAST)
}

var pkgAST = &ast.Package{
	BaseNode: ast.BaseNode{
		Comments: nil,
		Errors:   nil,
		Loc:      nil,
	},
	Files: []*ast.File{&ast.File{
		BaseNode: ast.BaseNode{
			Comments: nil,
			Errors:   nil,
			Loc: &ast.SourceLocation{
				End: ast.Position{
					Column: 18,
					Line:   6,
				},
				File:   "array.flux",
				Source: "package array\n\n\nimport \"array\"\n\nfrom = array.from",
				Start: ast.Position{
					Column: 1,
					Line:   1,
				},
			},
		},
		Body: []ast.Statement{&ast.VariableAssignment{
			BaseNode: ast.BaseNode{
				Comments: nil,
				Errors:   nil,
				Loc: &ast.SourceLocation{
					End: ast.Position{
						Column: 18,
						Line:   6,
					},
					File:   "array.flux",
					Source: "from = array.from",
					Start: ast.Position{
						Column: 1,
						Line:   6,
					},
				},
			},
			ID: &ast.Identifier{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 5,
							Line:   6,
						},
						File:   "array.flux",
						Source: "from",
						Start: ast.Position{
							Column: 1,
							Line:   6,
						},
					},
				},
				Name: "from",
			},
			Init: &ast.MemberExpression{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 18,
							Line:   6,
						},
						File:   "array.flux",
						Source: "array.from",
						Start: ast.Position{
							Column: 8,
							Line:   6,
						},
					},
				},
				Lbrack: nil,
				Object: &ast.Identifier{
					BaseNode: ast.BaseNode{
						Comments: nil,
						Errors:   nil,
						Loc: &ast.SourceLocation{
							End: ast.Position{
								Column: 13,
								Line:   6,
							},
							File:   "array.flux",
							Source: "array",
							Start: ast.Position{
								Column: 8,
								Line:   6,
							},
						},
					},
					Name: "array",
				},
				Property: &ast.Identifier{
					BaseNode: ast.BaseNode{
						Comments: nil,
						Errors:   nil,
						Loc: &ast.SourceLocation{
							End: ast.Position{
								Column: 18,
								Line:   6,
							},
							File:   "array.flux",
							Source: "from",
							Start: ast.Position{
								Column: 14,
								Line:   6,
							},
						},
					},
					Name: "from",
				},
				Rbrack: nil,
			},
		}},
		Eof: nil,
		Imports: []*ast.ImportDeclaration{&ast.ImportDeclaration{
			As: nil,
			BaseNode: ast.BaseNode{
				Comments: nil,
				Errors:   nil,
				Loc: &ast.SourceLocation{
					End: ast.Position{
						Column: 15,
						Line:   4,
					},
					File:   "array.flux",
					Source: "import \"array\"",
					Start: ast.Position{
						Column: 1,
						Line:   4,
					},
				},
			},
			Path: &ast.StringLiteral{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 15,
							Line:   4,
						},
						File:   "array.flux",
						Source: "\"array\"",
						Start: ast.Position{
							Column: 8,
							Line:   4,
						},
					},
				},
				Value: "array",
			},
		}},
		Metadata: "parser-type=rust",
		Name:     "array.flux",
		Package: &ast.PackageClause{
			BaseNode: ast.BaseNode{
				Comments: nil,
				Errors:   nil,
				Loc: &ast.SourceLocation{
					End: ast.Position{
						Column: 14,
						Line:   1,
					},
					File:   "array.flux",
					Source: "package array",
					Start: ast.Position{
						Column: 1,
						Line:   1,
					},
				},
			},
			Name: &ast.Identifier{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 14,
							Line:   1,
						},
						File:   "array.flux",
						Source: "array",
						Start: ast.Position{
							Column: 9,
							Line:   1,
						},
					},
				},
				Name: "array",
			},
		},
	}},
	Package: "array",
	Path:    "experimental/array",
}
