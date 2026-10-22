workspace "Indentation" {

	model {
		u = person "User"

          s = softwareSystem "System" {
	          api = container "API" "" "Kotlin"
  }
		u -> s "Uses"
	}

  views {
		systemContext s "context" {
                include *
		}
  }

}
