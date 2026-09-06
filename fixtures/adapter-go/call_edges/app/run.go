package app

import (
	"fmt"

	"example.com/callgraph/models"
)

func Run() string {
	// A cross-package call, which in Go is always written through the package
	// name. Skipping selectors would leave no edge crossing a file boundary.
	user := models.NewUser("Ada")

	// A composite literal in another package: construction.
	other := models.User{Name: "Grace"}

	// A method call on a value: recorded as a property access on the receiver,
	// never as a call edge to the type.
	greeting := user.Greeting()

	// Same-package call.
	local := describe(other)

	// A type conversion, spelled exactly like a call. Nothing is called, so
	// the resolved target's kind is what keeps this out of --kind calls.
	id := models.UserID(42)
	_ = id

	// A builtin and a third-party package: neither names a symbol here.
	total := len(greeting)
	fmt.Println(total)

	return local
}

func describe(u models.User) string {
	// The receiver's type is declared here, so the method call is recorded as
	// a property access on models.User rather than as a call edge.
	return u.Greeting() + u.Name
}
