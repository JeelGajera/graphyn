package models

type UserID int64

type User struct {
	Name string
}

func (u User) Greeting() string {
	return "hello " + u.Name
}

func NewUser(name string) User {
	return User{Name: name}
}

func UnusedHelper() {}
