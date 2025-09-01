;;; lbd-mode.el --- Major mode for Lambda-Discipline Language -*- lexical-binding: t; -*-
;;
;; Author: NinthCircle
;; Version: 0.1
;; Keywords: languages
;; URL: https://gitlab.com/ninthcircle/lambda-discipline/-/tree/master/editor-plugins/emacs/lbd-mode.el

(defvar lbd-mode-hook nil)

(defvar lbd-mode-map
  (let ((map (make-sparse-keymap)))
    map)
  "Keymap for lbd major mode")

;; Syntax table
(defvar lbd-mode-syntax-table
  (let ((st (make-syntax-table)))
    ;; Comment syntax: “--” starts a comment, newline ends it
    (modify-syntax-entry ?- ". 12" st)
    (modify-syntax-entry ?\n ">" st)

    ;; Parentheses
    (modify-syntax-entry ?\( "(" st)
    (modify-syntax-entry ?\) ")" st)

    ;; You can tweak these if you want string/char support later
    (modify-syntax-entry ?\" "\"" st)
    (modify-syntax-entry ?' "w" st)

    st)
  "Syntax table for lbd-mode.")

;; Comment settings
(setq-local comment-start "-- ")
(setq-local comment-end "")

;; Font lock
(defconst lbd--keywords
  '("true" "false" "if_then" "pair" "first" "second" "use")
  "Language keywords for lbd-mode.")

(defvar lbd-font-lock-keywords
  (let* ((kw (regexp-opt lbd--keywords 'words))
         ;; types are capitalized identifiers like Any Bool Float
         (type-re "\\b\\([A-Z][A-Za-z0-9_]*\\)\\b")
         ;; arrows
         (arrow-re "->")
         ;; lambda/backslash `\` and the dot that follows the argument list
         (lambda-re "\\\\[[:alnum:]_]+:\\s-*\\([A-Za-z0-9_<>]+\\)\\.\\|\\\\")
         ;; top-level defs: name : type =
         (def-re "^\\s-*\\([a-zA-Z_][a-zA-Z0-9_?']*\\)\\s-*:")
         )
    `(
      (,kw . font-lock-keyword-face)
      (,type-re 1 font-lock-type-face)
      (,arrow-re . font-lock-builtin-face)
      ("\\\\" . font-lock-builtin-face) ;; the backslash lambda
      ("\\." . font-lock-constant-face) ;; dots as separators
      ;; highlight top-level names like `name:` at bol
      (,def-re 1 font-lock-function-name-face)
      ))
  "Font lock patterns for lbd-mode.")

;; Indentation
(defun lbd--calculate-indent ()
  "Calculate indentation for current line in `lbd-mode'.
Returns number of spaces to indent to.
This implementation uses parenthesis depth as a baseline and adds
4 spaces for continued lines after a dot or after a lambda argument list."
  (save-excursion
    (beginning-of-line)
    (let* ((parse (syntax-ppss))
           (paren-depth (nth 0 parse))
           indent)
      (cond
       ;; If inside parens, align with the opening paren plus two spaces
       ((> paren-depth 0)
        (goto-char (nth 1 parse))
        (setq indent (+ (current-column) 2)))
       ;; If previous non-blank line ends with '.' or is a lambda start, indent
       (t
        (save-excursion
          (forward-line -1)
          (while (and (not (bobp)) (looking-at "^\\s-*$"))
            (forward-line -1))
          (end-of-line)
          (cond
           ((looking-back "\\\\.*\\.\\s-*" (line-beginning-position))
            (setq indent (+ (current-indentation) 4)))
           ((looking-back "\\\\" (line-beginning-position))
            (setq indent (+ (current-indentation) 4)))
           ((looking-back "\\.\\s-*" (line-beginning-position))
            (setq indent (+ (current-indentation) 4)))
           (t (setq indent 0))))))
      (max indent 0))))

(defun lbd-indent-line ()
  "Indent current line as lbd code."
  (interactive)
  (let ((indent (lbd--calculate-indent))
        (offset (- (current-column) (current-indentation))))
    (if indent
        (progn
          (indent-line-to indent)
          (when (> offset 0) (forward-char offset)))
      (indent-line-to 0))))

;; Imenu support
(defvar lbd-imenu-generic-expression
  '(( "Definitions" "^\\s-*\\([a-zA-Z_][a-zA-Z0-9_?']*\\)\\s-*:" 1))
  "Imenu expression for lbd-mode to index top-level definitions.")

;;;###autoload
(define-derived-mode lbd-mode prog-mode "LBD"
  "Major mode for editing Lambda-Discipline-like files."
  :syntax-table lbd-mode-syntax-table
  (setq-local font-lock-defaults '(lbd-font-lock-keywords))
  ;; comment style
  (setq-local comment-start "-- ")
  (setq-local comment-end "")
  ;; indentation
  (setq-local indent-line-function #'lbd-indent-line)
  ;; imenu
  (setq-local imenu-generic-expression lbd-imenu-generic-expression))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.lbd\\'" . lbd-mode))
(add-to-list 'auto-mode-alist '("\\.lambda\\'" . lbd-mode))

(provide 'lbd-mode)
;;; lbd-mode.el ends here
