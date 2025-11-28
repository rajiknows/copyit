package lib

import (
	models "copytrading/apiserver/internal"

	"github.com/gofiber/fiber/v2"
)


func CreateCustomer(c *fiber.Ctx) {
	var customer models.User
	if err := c.(&customer); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	// Check if email already exists
	var existingCustomer models.Customer
	if err := database.DB.Where("email = ?", customer.Email).First(&existingCustomer).Error; err == nil {
		c.JSON(http.StatusConflict, gin.H{"error": "Email already exists"})
		return
	} else if err != gorm.ErrRecordNotFound {
		// Log database errors
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error", "details": err.Error()})
		return
	}

	// Set the password to the email
	customer.Password = customer.Email

	// Save the customer
	if err := database.DB.Create(&customer).Error; err != nil {
		// Log creation errors
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Could not create customer", "details": err.Error()})
		return
	}

	c.JSON(http.StatusOK, customer)
}
