package authhandler

import (
	"copytrading/apiserver/internal/lib"

	"github.com/gofiber/fiber/v2"
)

func Login(c *fiber.Ctx)error{
	return nil
}

type RegisterBody struct{
	Email string `json:"email"`
	Password string `json:"password"`
}

func Register(c *fiber.Ctx) error{
	p := new(RegisterBody)
	if err:= c.BodyParser(p); err!=nil{
		return  err
	}

	// check if the user already exists
	// if not
	// send an email verification link

	lib.CreateCustomer(c)



	return nil
}
