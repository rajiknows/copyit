package lib

import (
	"math"

	"github.com/gofiber/fiber/v2"
)

type H map[string]any

func SuccessResponseJSON(c *fiber.Ctx, obj any, msg ...string) error {
	resMsg := "Success."
	if len(msg) > 0 {
		resMsg = msg[0]
	}

	return c.Status(fiber.StatusOK).JSON(H{
		"message": resMsg,
		"data":    obj,
	})
}

func PageSuccessResponseJSON[T any](c *fiber.Ctx, objs []T, currentPage, pageSize int, totalCount int64, msg ...string) error {
	resMsg := "Success."
	if len(msg) > 0 {
		resMsg = msg[0]
	}

	return c.Status(fiber.StatusOK).JSON(H{
		"message":      resMsg,
		"data":         objs,
		"count":        len(objs),
		"current_page": currentPage,
		"total_pages":  int(math.Ceil(float64(totalCount) / float64(pageSize))),
	})
}

func ErrorResponseJSON(c *fiber.Ctx, status int, message string, detail ...any) error {
	var detailObj any
	if len(detail) > 0 {
		detailObj = detail[0]
	}

	// optional logging (keep your logger call)
	if status >= 500 {
		LibLogger.Println(LoggerLabelError, "(INTERNAL SERVER ERROR)", message, detailObj)
	} else if status >= 400 {
		LibLogger.Println(LoggerLabelWarning, message, detailObj)
	}

	return c.Status(status).JSON(H{
		"message": message,
		"detail":  detailObj,
	})
}
