package api

// `_test.go` is the Go compiler's own rule for what is a test.
func TestServe(t interface{}) int {
	return Serve()
}
