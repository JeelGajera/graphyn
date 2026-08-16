package handlers

import (
	"fmt"

	m "github.com/test/features/models"
	"github.com/test/features/store"
)

type UserHandler struct {
	reader store.Reader
}

func NewUserHandler(reader store.Reader) *UserHandler {
	return &UserHandler{reader: reader}
}

func (h *UserHandler) Describe(payload *m.UserPayload, order *m.Order) string {
	fmt.Println("describing")
	return payload.Email + payload.UserID + order.OrderID
}
