package svc

import "github.com/test/regression/models"

func UseAlpha(whatever *models.Alpha) string { return whatever.AField }
func UseBeta(anything *models.Beta) string   { return anything.BField }
