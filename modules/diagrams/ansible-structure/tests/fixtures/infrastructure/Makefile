.PHONY: lint deploy

lint:
	ansible-lint site.yml

deploy:
	ansible-playbook -i inventories/production site.yml
