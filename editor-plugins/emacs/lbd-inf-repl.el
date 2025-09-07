(require 'comint)

(defvar lbd-repl-buffer "*lbd*"
  "Buffer name for the lambda-discipline REPL.")

(defvar lbd-root-dir
  (expand-file-name "../../"
                    (file-name-directory (or load-file-name buffer-file-name)))
  "Root directory of the lambda-discipline project.")

(defun lbd-run-inf-repl ()
  "Run an inferior lambda-discipline REPL inside Emacs, rooted at `lbd-root-dir`."
  (interactive)
  (let* ((default-directory lbd-root-dir) ; set working dir
         (exe (expand-file-name "./cmake-build-debug/lbd.exe" lbd-root-dir)))
    (unless (comint-check-proc lbd-repl-buffer)
      (set-buffer
       (apply #'make-comint "lbd" exe nil '("--repl"))))
    (pop-to-buffer lbd-repl-buffer)))

(defun lbd-send-region (start end)
  "Send the current region to the lbd process."
  (interactive "r")
  (comint-send-region (get-buffer-process lbd-repl-buffer) start end)
  (comint-send-string (get-buffer-process lbd-repl-buffer) "\n"))

(define-key global-map (kbd "C-c C-e") #'lbd-send-region)
