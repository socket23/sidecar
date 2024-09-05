In our repository I want to understand how we are handling non-language-config files, give me a list of the references I should look at?

get_symbol_change_set is not using the proper outline nodes which is via the editor, on the editor side we have to make sure that we just get the data no matter what and we are good

Added support for any kind of language with file_content ts_language_config in `chunking/languages.rs`
Its pretty cool cause its just a single node to support any kind of file
