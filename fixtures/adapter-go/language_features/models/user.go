package models

type Base struct {
	ID string
}

type UserPayload struct {
	Base
	UserID    string
	Email     string
	Timestamp string
}
