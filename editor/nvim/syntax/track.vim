" Track — Vim 8 / legacy syntax (flat, simple)
" Place in ~/.vim/syntax/track.vim and ~/.vim/ftdetect/track.vim,
" or source directly:  :source /path/to/track.vim
if exists('b:current_syntax') | finish | endif

" keywords
syntax keyword trackKeyword import use fn return if else while for in let mut with struct enum union match const type as
syntax keyword trackBoolean true false
syntax keyword trackMacroDef @macro
syntax match trackMacro /@[a-zA-Z_][a-zA-Z0-9_]*/

" types
syntax keyword trackType i8 u8 i32 u32 i64 u64 bool void ptr
syntax match trackTypeName /\<[A-Z][a-zA-Z0-9_]*\>/

" functions
syntax match trackFunction /\v<[a-z_][a-zA-Z0-9_]*\ze\s*\(/

" strings & escapes
syntax region trackString start=/"/ skip=/\\./ end=/"/ contains=trackEscape
syntax match trackEscape /\\[\"\\nrt0']/ contained
syntax match trackEscape /\\x[0-9A-Fa-f]\{2}/ contained

" comments
syntax match trackComment /\/\/.*$/ contains=trackTodo
syntax region trackComment start=/\/\*/ end=/\*\// contains=trackTodo
syntax keyword trackTodo TODO FIXME NOTE HACK contained

" numbers
syntax match trackNumber /\<0[xX][0-9a-fA-F_]\+\>/
syntax match trackNumber /\<0[bB][01_]\+\>/
syntax match trackNumber /\<[0-9][0-9_]*\>/

" operators / punctuation
syntax match trackOperator /->\|=>\|::\|\.\.\|&&\|||/
syntax match trackOperator /[+\-*\/%<>=!&|^~]\+/
syntax match trackDelimiter /[{}()\[\],;:\.]/

highlight default link trackKeyword Keyword
highlight default link trackBoolean Boolean
highlight default link trackMacro Macro
highlight default link trackMacroDef Define
highlight default link trackType Type
highlight default link trackTypeName Structure
highlight default link trackFunction Function
highlight default link trackString String
highlight default link trackEscape SpecialChar
highlight default link trackComment Comment
highlight default link trackTodo Todo
highlight default link trackNumber Number
highlight default link trackOperator Operator
highlight default link trackDelimiter Delimiter

let b:current_syntax = 'track'
