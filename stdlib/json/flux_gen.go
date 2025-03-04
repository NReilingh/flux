// DO NOT EDIT: This file is autogenerated via the builtin command.

package json

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
					Column: 15,
					Line:   27,
				},
				File:   "json.flux",
				Source: "package json\n\n\n// encode converts a value into JSON bytes\n// Time values are encoded using RFC3339.\n// Duration values are encoded in number of milleseconds since the epoch.\n// Regexp values are encoded as their string representation.\n// Bytes values are encodes as base64-encoded strings.\n// Function values cannot be encoded and will produce an error.\n//\n// ## Parameters\n// - `V` is the value to convert\n//\n// ## Encode all values in a column in JSON bytes\n//\n// ```\n// import \"json\"\n//\n// from(bucket: \"example-bucket\")\n//   |> range(start: -1h)\n//   |> map(fn: (r) => ({\n//       r with _value: json.encode(v: r._value)\n//   }))\n// ```\n//\nbuiltin encode",
				Start: ast.Position{
					Column: 1,
					Line:   2,
				},
			},
		},
		Body: []ast.Statement{&ast.BuiltinStatement{
			BaseNode: ast.BaseNode{
				Comments: []ast.Comment{ast.Comment{Text: "// encode converts a value into JSON bytes\n"}, ast.Comment{Text: "// Time values are encoded using RFC3339.\n"}, ast.Comment{Text: "// Duration values are encoded in number of milleseconds since the epoch.\n"}, ast.Comment{Text: "// Regexp values are encoded as their string representation.\n"}, ast.Comment{Text: "// Bytes values are encodes as base64-encoded strings.\n"}, ast.Comment{Text: "// Function values cannot be encoded and will produce an error.\n"}, ast.Comment{Text: "//\n"}, ast.Comment{Text: "// ## Parameters\n"}, ast.Comment{Text: "// - `V` is the value to convert\n"}, ast.Comment{Text: "//\n"}, ast.Comment{Text: "// ## Encode all values in a column in JSON bytes\n"}, ast.Comment{Text: "//\n"}, ast.Comment{Text: "// ```\n"}, ast.Comment{Text: "// import \"json\"\n"}, ast.Comment{Text: "//\n"}, ast.Comment{Text: "// from(bucket: \"example-bucket\")\n"}, ast.Comment{Text: "//   |> range(start: -1h)\n"}, ast.Comment{Text: "//   |> map(fn: (r) => ({\n"}, ast.Comment{Text: "//       r with _value: json.encode(v: r._value)\n"}, ast.Comment{Text: "//   }))\n"}, ast.Comment{Text: "// ```\n"}, ast.Comment{Text: "//\n"}},
				Errors:   nil,
				Loc: &ast.SourceLocation{
					End: ast.Position{
						Column: 15,
						Line:   27,
					},
					File:   "json.flux",
					Source: "builtin encode",
					Start: ast.Position{
						Column: 1,
						Line:   27,
					},
				},
			},
			Colon: nil,
			ID: &ast.Identifier{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 15,
							Line:   27,
						},
						File:   "json.flux",
						Source: "encode",
						Start: ast.Position{
							Column: 9,
							Line:   27,
						},
					},
				},
				Name: "encode",
			},
			Ty: ast.TypeExpression{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 33,
							Line:   27,
						},
						File:   "json.flux",
						Source: "(v: A) => bytes",
						Start: ast.Position{
							Column: 18,
							Line:   27,
						},
					},
				},
				Constraints: []*ast.TypeConstraint{},
				Ty: &ast.FunctionType{
					BaseNode: ast.BaseNode{
						Comments: nil,
						Errors:   nil,
						Loc: &ast.SourceLocation{
							End: ast.Position{
								Column: 33,
								Line:   27,
							},
							File:   "json.flux",
							Source: "(v: A) => bytes",
							Start: ast.Position{
								Column: 18,
								Line:   27,
							},
						},
					},
					Parameters: []*ast.ParameterType{&ast.ParameterType{
						BaseNode: ast.BaseNode{
							Comments: nil,
							Errors:   nil,
							Loc: &ast.SourceLocation{
								End: ast.Position{
									Column: 23,
									Line:   27,
								},
								File:   "json.flux",
								Source: "v: A",
								Start: ast.Position{
									Column: 19,
									Line:   27,
								},
							},
						},
						Kind: "Required",
						Name: &ast.Identifier{
							BaseNode: ast.BaseNode{
								Comments: nil,
								Errors:   nil,
								Loc: &ast.SourceLocation{
									End: ast.Position{
										Column: 20,
										Line:   27,
									},
									File:   "json.flux",
									Source: "v",
									Start: ast.Position{
										Column: 19,
										Line:   27,
									},
								},
							},
							Name: "v",
						},
						Ty: &ast.TvarType{
							BaseNode: ast.BaseNode{
								Comments: nil,
								Errors:   nil,
								Loc: &ast.SourceLocation{
									End: ast.Position{
										Column: 23,
										Line:   27,
									},
									File:   "json.flux",
									Source: "A",
									Start: ast.Position{
										Column: 22,
										Line:   27,
									},
								},
							},
							ID: &ast.Identifier{
								BaseNode: ast.BaseNode{
									Comments: nil,
									Errors:   nil,
									Loc: &ast.SourceLocation{
										End: ast.Position{
											Column: 23,
											Line:   27,
										},
										File:   "json.flux",
										Source: "A",
										Start: ast.Position{
											Column: 22,
											Line:   27,
										},
									},
								},
								Name: "A",
							},
						},
					}},
					Return: &ast.NamedType{
						BaseNode: ast.BaseNode{
							Comments: nil,
							Errors:   nil,
							Loc: &ast.SourceLocation{
								End: ast.Position{
									Column: 33,
									Line:   27,
								},
								File:   "json.flux",
								Source: "bytes",
								Start: ast.Position{
									Column: 28,
									Line:   27,
								},
							},
						},
						ID: &ast.Identifier{
							BaseNode: ast.BaseNode{
								Comments: nil,
								Errors:   nil,
								Loc: &ast.SourceLocation{
									End: ast.Position{
										Column: 33,
										Line:   27,
									},
									File:   "json.flux",
									Source: "bytes",
									Start: ast.Position{
										Column: 28,
										Line:   27,
									},
								},
							},
							Name: "bytes",
						},
					},
				},
			},
		}},
		Eof:      nil,
		Imports:  nil,
		Metadata: "parser-type=rust",
		Name:     "json.flux",
		Package: &ast.PackageClause{
			BaseNode: ast.BaseNode{
				Comments: []ast.Comment{ast.Comment{Text: "// Package json functions provide tools for working with JSON.\n"}},
				Errors:   nil,
				Loc: &ast.SourceLocation{
					End: ast.Position{
						Column: 13,
						Line:   2,
					},
					File:   "json.flux",
					Source: "package json",
					Start: ast.Position{
						Column: 1,
						Line:   2,
					},
				},
			},
			Name: &ast.Identifier{
				BaseNode: ast.BaseNode{
					Comments: nil,
					Errors:   nil,
					Loc: &ast.SourceLocation{
						End: ast.Position{
							Column: 13,
							Line:   2,
						},
						File:   "json.flux",
						Source: "json",
						Start: ast.Position{
							Column: 9,
							Line:   2,
						},
					},
				},
				Name: "json",
			},
		},
	}},
	Package: "json",
	Path:    "json",
}
